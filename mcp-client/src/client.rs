use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionRequestArgs,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub api_key: String,
    pub server_path: PathBuf,
    pub scripts_dir: PathBuf,
    pub model: String,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct InteractionResult {
    pub generated_script: String,
    pub initialize_response: String,
    pub tool_response: String,
}

pub async fn run_interaction(opts: ClientOptions) -> anyhow::Result<InteractionResult> {
    if opts.api_key.trim().is_empty() {
        return Err(anyhow!("OpenAI API key is empty"));
    }

    let config = OpenAIConfig::new().with_api_key(opts.api_key.clone());
    let openai = Client::with_config(config);
    let script = generate_applescript(&openai, &opts.model, &opts.prompt).await?;

    let mut server = McpServerProcess::spawn(&opts.server_path, &opts.scripts_dir).await?;

    server
        .send_request(
            1,
            "initialize",
            json!({
                "client": {"name": "mcp-client", "version": env!("CARGO_PKG_VERSION")},
                "protocol_version": "2024-10-30"
            }),
        )
        .await?;
    let init_response = server.read_response().await?;

    server
        .send_request(
            2,
            "tools/call",
            json!({
                "name": "app.finder",
                "arguments": {"script": script.clone()}
            }),
        )
        .await?;
    let tool_response = server.read_response().await?;

    server.shutdown().await;

    Ok(InteractionResult {
        generated_script: script,
        initialize_response: init_response,
        tool_response,
    })
}

async fn generate_applescript(
    client: &Client<OpenAIConfig>,
    model: &str,
    prompt: &str,
) -> anyhow::Result<String> {
    let request = CreateChatCompletionRequestArgs::default()
        .model(model)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You write short AppleScript bodies that can run inside a `tell application` block. Respond with AppleScript code only, no explanations.".to_string())
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(ChatCompletionRequestUserMessageContent::Text(prompt.to_string()))
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    let choice = response
        .choices
        .first()
        .ok_or_else(|| anyhow!("OpenAI response contained no choices"))?;
    let message = choice
        .message
        .content
        .as_deref()
        .ok_or_else(|| anyhow!("OpenAI response contained no message content"))?;
    Ok(message.trim().to_string())
}

struct McpServerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpServerProcess {
    async fn spawn(server_path: &Path, scripts_dir: &Path) -> anyhow::Result<Self> {
        let mut command = Command::new(server_path);
        command
            .arg("--transport")
            .arg("stdio")
            .arg("--scripts-dir")
            .arg(scripts_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = command.spawn().context("Failed to spawn MCP server")?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture server stdin"))?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| anyhow!("failed to capture server stdout"))?,
        );

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    async fn send_request(&mut self, id: u64, method: &str, params: Value) -> anyhow::Result<()> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let body = serde_json::to_string(&payload)?;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        self.stdin.write_all(frame.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_response(&mut self) -> anyhow::Result<String> {
        let mut header_line = String::new();
        let mut content_length: Option<usize> = None;

        loop {
            header_line.clear();
            let bytes = self.stdout.read_line(&mut header_line).await?;
            if bytes == 0 {
                return Err(anyhow!("Unexpected EOF while reading response header"));
            }
            if header_line == "\r\n" {
                break;
            }
            if let Some(rest) = header_line.trim().strip_prefix("Content-Length:") {
                content_length = Some(rest.trim().parse()?);
            }
        }

        let len = content_length.ok_or_else(|| anyhow!("Missing Content-Length header"))?;
        let mut buffer = vec![0u8; len];
        self.stdout.read_exact(&mut buffer).await?;
        let payload = String::from_utf8(buffer)?;
        Ok(payload)
    }

    async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}
