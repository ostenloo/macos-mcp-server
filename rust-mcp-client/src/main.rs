use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use rust_mcp_client::client::{ClientOptions, run_interaction};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Sample MCP client that uses OpenAI to drive AppleScript tools"
)]
struct Cli {
    /// Path to the MCP server executable.
    #[arg(
        long,
        value_name = "PATH",
        default_value = "../rust-mcp-server/target/debug/rust-mcp-server"
    )]
    server_path: PathBuf,

    /// Directory containing exported AppleScript dictionaries.
    #[arg(long, value_name = "DIR", default_value = "../AppScripts")]
    scripts_dir: PathBuf,

    /// OpenAI model to use when generating AppleScript snippets.
    #[arg(long, default_value = "gpt-4.1-mini")]
    model: String,

    /// Prompt describing the automation you want the LLM to translate into AppleScript.
    #[arg(long, default_value = "Return the name of the front Finder window.")]
    prompt: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let api_key = std::env::var("OPENAI_API_KEY")
        .context("Set the OPENAI_API_KEY environment variable to your OpenAI API key")?;

    let result = run_interaction(ClientOptions {
        api_key,
        server_path: cli.server_path,
        scripts_dir: cli.scripts_dir,
        model: cli.model,
        prompt: cli.prompt,
    })
    .await?;

    println!(
        "Generated AppleScript script:\n{}\n",
        result.generated_script.trim()
    );
    println!("Initialize response:\n{}\n", result.initialize_response);
    println!("tools/call response:\n{}\n", result.tool_response);

    Ok(())
}
