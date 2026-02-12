use anyhow::Result;
use clap::{Parser, Subcommand};

mod cli;
mod indexer;
mod mcp;
mod output;
mod server;
mod storage;

#[derive(Parser)]
#[command(name = "ctxhelpr", about = "Semantic code indexing for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server (called by Claude Code via stdio)
    Serve,
    /// Set up ctxhelpr integration with Claude Code
    Setup,
    /// Remove ctxhelpr integration from Claude Code
    Uninstall,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Serve => server::run().await,
        Commands::Setup => cli::setup::run(),
        Commands::Uninstall => cli::uninstall::run(),
    }
}
