mod config;
mod protocol;
mod server;
mod state;
mod tools;
mod transport;

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use config::Cli;
use tools::load_tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialise_tracing();

    let cli = Cli::parse();
    let transport_config = cli.transport_config()?;
    let tool_registry = load_tools(&cli.scripts_dir)?;
    let transport = transport::create_transport(&transport_config).await?;
    let state = state::AppState::default();
    let server = server::Server::new(transport, state, tool_registry);

    server.run().await
}

fn initialise_tracing() {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("failed to build log filter");

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}
