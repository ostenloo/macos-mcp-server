mod stdio;

use async_trait::async_trait;
use std::path::PathBuf;

use crate::config::TransportConfig;

pub use stdio::StdioTransport;

/// Abstract interface for the JSON-RPC transport used by the server.
#[async_trait]
pub trait Transport: Send {
    /// Reads the next complete JSON-RPC payload.
    async fn read(&mut self) -> anyhow::Result<Option<String>>;
    /// Writes a JSON-RPC payload to the peer.
    async fn write(&mut self, payload: &str) -> anyhow::Result<()>;
}

/// Helper alias for boxed transport trait objects with the right bounds.
pub type BoxTransport = Box<dyn Transport + Send>;

/// Factory to create the desired transport from configuration.
pub async fn create_transport(config: &TransportConfig) -> anyhow::Result<BoxTransport> {
    let transport: BoxTransport = match config {
        TransportConfig::Stdio => Box::new(StdioTransport::new()),
        TransportConfig::UnixSocket(path) => {
            let path: PathBuf = path.clone();
            return Err(anyhow::anyhow!(
                "Unix domain socket transport is not implemented yet (requested path: {path:?})"
            ));
        }
    };

    Ok(transport)
}
