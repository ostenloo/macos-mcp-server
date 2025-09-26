use std::sync::Arc;

use tokio::sync::RwLock;

/// Shared mutable state for the MCP server.
#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<StateInner>>,
}

#[derive(Default)]
struct StateInner {
    /// Example stored configuration state; extend with whatever your server needs.
    pub initialized: bool,
}

impl AppState {
    pub async fn mark_initialized(&self) {
        let mut inner = self.inner.write().await;
        inner.initialized = true;
    }

    pub async fn is_initialized(&self) -> bool {
        self.inner.read().await.initialized
    }
}
