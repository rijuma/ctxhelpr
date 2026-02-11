use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand};

mod cli;
mod indexer;
mod mcp;
mod output;
mod server;
mod storage;

use cli::Scope;

#[derive(Parser)]
#[command(name = "ctxhelpr", about = "Semantic code indexing for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Internal: MCP server started automatically by Claude Code (not for manual use)
    Serve,
    /// Set up ctxhelpr integration with Claude Code
    Setup(ScopeArgs),
    /// Remove ctxhelpr integration from Claude Code
    Uninstall(ScopeArgs),
    /// Manage ctxhelpr tool permissions in Claude Code
    Perms(PermsArgs),
}

#[derive(Args)]
struct ScopeArgs {
    /// Install to local project (.claude/)
    #[arg(short = 'l', long, conflicts_with = "global")]
    local: bool,
    /// Install to global ~/.claude/
    #[arg(short = 'g', long, conflicts_with = "local")]
    global: bool,
}

impl ScopeArgs {
    fn into_scope(self) -> Scope {
        match (self.local, self.global) {
            (true, _) => Scope::Local,
            (_, true) => Scope::Global,
            _ => Scope::Unspecified,
        }
    }
}

#[derive(Args)]
struct PermsArgs {
    /// Apply to local project (.claude/)
    #[arg(short = 'l', long, conflicts_with = "global")]
    local: bool,
    /// Apply to global ~/.claude/
    #[arg(short = 'g', long, conflicts_with = "local")]
    global: bool,
    /// Grant all ctxhelpr tool permissions
    #[arg(short = 'a', long, conflicts_with = "remove")]
    all: bool,
    /// Revoke all ctxhelpr tool permissions
    #[arg(short = 'r', long, conflicts_with = "all")]
    remove: bool,
}

impl PermsArgs {
    fn scope(&self) -> Scope {
        match (self.local, self.global) {
            (true, _) => Scope::Local,
            (_, true) => Scope::Global,
            _ => Scope::Unspecified,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Serve) => server::run().await,
        Some(Commands::Setup(args)) => cli::setup::run(args.into_scope()),
        Some(Commands::Uninstall(args)) => cli::uninstall::run(args.into_scope()),
        Some(Commands::Perms(args)) => cli::perms::run(args.scope(), args.all, args.remove),
        None => {
            Cli::command().print_help()?;
            Ok(())
        }
    }
}
