use serde::{Deserialize, Serialize};

/// Current MCP protocol version supported by this server skeleton.
pub const PROTOCOL_VERSION: &str = "2024-10-30";

/// Generic JSON-RPC request envelope used by the protocol.
#[derive(Debug, Deserialize)]
pub struct RequestEnvelope {
    #[serde(default = "jsonrpc_version", rename = "jsonrpc")]
    pub protocol: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Generic JSON-RPC response envelope used by the protocol.
#[derive(Debug, Serialize)]
pub struct ResponseEnvelope {
    #[serde(rename = "jsonrpc")]
    pub protocol: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

impl ResponseEnvelope {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            protocol: jsonrpc_version(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, error: ResponseError) -> Self {
        Self {
            protocol: jsonrpc_version(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC structured error payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Parameters for the `initialize` request defined by the MCP spec.
#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub client: ClientIdentity,
    #[serde(default)]
    pub capabilities: ClientCapabilities,
    #[serde(default)]
    pub protocol_version: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub experimental: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ClientIdentity {
    pub name: String,
    pub version: Option<String>,
}

/// Result payload returned from a successful `initialize` response.
#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolDescription>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub resources: Vec<ResourceDescription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ToolListParams {
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolListResult {
    pub tools: Vec<ToolDescription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolResultContent>,
}

#[derive(Debug, Serialize)]
pub struct ToolResultContent {
    #[serde(rename = "type")]
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ResourceDescription {
    pub uri: String,
    pub description: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn jsonrpc_version() -> String {
    "2.0".to_string()
}
