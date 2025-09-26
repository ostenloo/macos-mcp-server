use anyhow::{Context, anyhow};
use serde_json::json;
use tracing::{debug, info, warn};

use crate::protocol::{
    InitializeParams, InitializeResult, PROTOCOL_VERSION, RequestEnvelope, ResponseEnvelope,
    ResponseError, ServerCapabilities, ServerInfo, ToolCallParams, ToolCallResult, ToolListParams,
    ToolListResult, ToolResultContent,
};
use crate::state::AppState;
use crate::tools::{Tool, ToolRegistry};
use crate::transport::BoxTransport;
use tokio::process::Command;

/// Main MCP server type that owns state, capabilities, and handles JSON-RPC traffic.
pub struct Server {
    transport: BoxTransport,
    state: AppState,
    capabilities: ServerCapabilities,
    info: ServerInfo,
    tool_registry: ToolRegistry,
}

impl Server {
    pub fn new(transport: BoxTransport, state: AppState, tool_registry: ToolRegistry) -> Self {
        let capabilities = ServerCapabilities {
            tools: tool_registry.descriptions(),
            ..ServerCapabilities::default()
        };

        let info = ServerInfo {
            name: "rust-mcp-server".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            description: Some("Rust skeleton MCP server".into()),
        };

        Self {
            transport,
            state,
            capabilities,
            info,
            tool_registry,
        }
    }

    /// Entry point that pumps requests from the chosen transport until EOF.
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("starting MCP server");
        while let Some(frame) = self.transport.read().await? {
            debug!(payload = frame, "received frame");
            match serde_json::from_str::<RequestEnvelope>(&frame) {
                Ok(request) => {
                    if let Some(id) = request.id.clone() {
                        if let Err(err) = self.handle_request(id, request).await {
                            warn!(?err, "failed to handle request");
                        }
                    } else if let Err(err) = self.handle_notification(request).await {
                        warn!(?err, "failed to handle notification");
                    }
                }
                Err(err) => {
                    warn!(?err, "failed to deserialize request");
                }
            }
        }
        info!("transport closed; shutting down");
        Ok(())
    }

    async fn handle_request(
        &mut self,
        id: serde_json::Value,
        request: RequestEnvelope,
    ) -> anyhow::Result<()> {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(id, request.params).await,
            "ping" => self.handle_ping(id, request.params).await,
            "tools/list" => self.handle_tools_list(id, request.params).await,
            "tools/call" => self.handle_tools_call(id, request.params).await,
            method => {
                let response = ResponseEnvelope::error(
                    id,
                    ResponseError {
                        code: -32601,
                        message: format!("method '{method}' not implemented"),
                        data: None,
                    },
                );
                self.write_response(response).await
            }
        }
    }

    async fn handle_notification(&mut self, request: RequestEnvelope) -> anyhow::Result<()> {
        match request.method.as_str() {
            "shutdown" => {
                info!("client requested shutdown");
                // Future work: trigger graceful shutdown state.
                Ok(())
            }
            method => {
                debug!(method, "ignoring unsupported notification");
                Ok(())
            }
        }
    }

    async fn handle_initialize(
        &mut self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        if self.state.is_initialized().await {
            let response = ResponseEnvelope::error(
                id,
                ResponseError {
                    code: -32600,
                    message: "initialize already called".to_string(),
                    data: None,
                },
            );
            return self.write_response(response).await;
        }

        let params: InitializeParams =
            serde_json::from_value(params).context("failed to deserialize initialize params")?;
        info!(client = %params.client.name, "initializing session");
        self.state.mark_initialized().await;

        let result = InitializeResult {
            protocol_version: params
                .protocol_version
                .unwrap_or_else(|| PROTOCOL_VERSION.to_string()),
            capabilities: ServerCapabilities {
                tools: self.tool_registry.descriptions(),
                ..self.capabilities.clone()
            },
            server_info: self.info.clone(),
        };

        let response = ResponseEnvelope::success(id, serde_json::to_value(result)?);
        self.write_response(response).await
    }

    async fn handle_ping(
        &mut self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        let message = params
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("pong");
        let response = ResponseEnvelope::success(id, json!({ "message": message }));
        self.write_response(response).await
    }

    async fn handle_tools_list(
        &mut self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        let _params: ToolListParams = serde_json::from_value(params).unwrap_or_default();
        let result = ToolListResult {
            tools: self.tool_registry.descriptions(),
            next_cursor: None,
        };
        let response = ResponseEnvelope::success(id, serde_json::to_value(result)?);
        self.write_response(response).await
    }

    async fn handle_tools_call(
        &mut self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        let params: ToolCallParams =
            serde_json::from_value(params).context("failed to deserialize tools/call params")?;
        let tool = self
            .tool_registry
            .get(&params.name)
            .ok_or_else(|| anyhow!("unknown tool `{}`", params.name))?;

        let script = params
            .arguments
            .get("script")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("tool `{}` requires a `script` string argument", params.name))?;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(build_applescript(tool, script))
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let response = ResponseEnvelope::error(
                id,
                ResponseError {
                    code: -32010,
                    message: format!("tool `{}` execution failed", params.name),
                    data: Some(json!({
                        "stderr": stderr,
                        "status": output.status.code(),
                    })),
                },
            );
            return self.write_response(response).await;
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let result = ToolCallResult {
            content: vec![ToolResultContent {
                r#type: "text".into(),
                text: stdout,
            }],
        };
        let response = ResponseEnvelope::success(id, serde_json::to_value(result)?);
        self.write_response(response).await
    }

    async fn write_response(&mut self, response: ResponseEnvelope) -> anyhow::Result<()> {
        let payload = serde_json::to_string(&response)?;
        self.transport.write(&payload).await
    }
}

fn build_applescript(tool: &Tool, script: &str) -> String {
    let mut block = String::new();
    block.push_str(&format!("tell application \"{}\"\n", tool.app_name));
    block.push_str(script);
    if !script.ends_with('\n') {
        block.push('\n');
    }
    block.push_str("end tell\n");
    block
}
