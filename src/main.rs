use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand};

mod cli;
mod config;
mod indexer;
mod mcp;
mod output;
mod server;
mod storage;

use cli::Scope;
use cli::config_cmd::ConfigArgs;
use cli::repos::ReposCommands;

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
    /// Enable ctxhelpr integration with Claude Code
    Enable(ScopeArgs),
    /// Disable ctxhelpr integration from Claude Code
    Disable(ScopeArgs),
    /// Manage ctxhelpr tool permissions in Claude Code
    Perms(PermsArgs),
    /// Manage project configuration (.ctxhelpr.json)
    Config(ConfigArgs),
    /// Manage indexed repositories
    Repos {
        #[command(subcommand)]
        command: ReposCommands,
    },
    /// Update ctxhelpr to the latest version
    Update,
    /// Completely uninstall ctxhelpr (disable + remove binary)
    Uninstall,
}

#[derive(Args)]
struct ScopeArgs {
    /// Enable for local project (.claude/)
    #[arg(short = 'l', long, conflicts_with = "global")]
    local: bool,
    /// Enable for global ~/.claude/
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
        Some(Commands::Enable(args)) => cli::enable::run(args.into_scope()),
        Some(Commands::Disable(args)) => cli::disable::run(args.into_scope()),
        Some(Commands::Perms(args)) => cli::perms::run(args.scope(), args.all, args.remove),
        Some(Commands::Config(args)) => cli::config_cmd::run(args),
        Some(Commands::Repos { command }) => cli::repos::run(command),
        Some(Commands::Update) => cli::update::run(),
        Some(Commands::Uninstall) => cli::uninstall::run(),
        None => {
            Cli::command().print_help()?;
            Ok(())
        }
    }
}
