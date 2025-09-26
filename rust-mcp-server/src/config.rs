use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Command line configuration for the MCP server executable.
#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Rust reference implementation skeleton for an MCP server"
)]
pub struct Cli {
    /// Which transport layer to use for JSON-RPC framing.
    #[arg(long, value_enum, default_value_t = TransportKind::Stdio)]
    pub transport: TransportKind,

    /// Path to a Unix domain socket to listen on (used with `--transport unix-socket`).
    #[arg(long)]
    pub socket_path: Option<PathBuf>,

    /// Location of exported AppleScript dictionaries.
    #[arg(long, value_name = "DIR", default_value = "../AppScripts")]
    pub scripts_dir: PathBuf,
}

/// Supported transport types.
#[derive(Clone, Debug, Copy, Eq, PartialEq, ValueEnum)]
pub enum TransportKind {
    /// Standard input/output framing using MCP's Content-Length headers.
    Stdio,
    /// Unix domain socket (planned, not yet implemented).
    #[value(name = "unix-socket")]
    UnixSocket,
}

impl Cli {
    /// Validate the configuration and return the desired transport kind and ancillary data.
    pub fn transport_config(&self) -> anyhow::Result<TransportConfig> {
        match self.transport {
            TransportKind::Stdio => Ok(TransportConfig::Stdio),
            TransportKind::UnixSocket => {
                let path = self.socket_path.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("--socket-path is required when using --transport unix-socket")
                })?;
                Ok(TransportConfig::UnixSocket(path.clone()))
            }
        }
    }
}

/// Normalised transport configuration produced from CLI options.
#[derive(Clone, Debug)]
pub enum TransportConfig {
    Stdio,
    UnixSocket(PathBuf),
}
