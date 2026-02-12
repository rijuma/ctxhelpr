use anyhow::Result;
use std::fs;
use std::process::Command;

use super::claude_dir;

const SKILL_CONTENT: &str = include_str!("../assets/skill.md");
const INDEX_COMMAND_CONTENT: &str = include_str!("../assets/index_command.md");

fn binary_path() -> Result<String> {
    std::env::current_exe()?
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Binary path is not valid UTF-8"))
}

pub fn run() -> Result<()> {
    println!("Setting up ctxhelpr for Claude Code...\n");

    // 1. Register MCP server
    let bin = binary_path()?;
    print!("  Registering MCP server... ");
    let status = Command::new("claude")
        .args([
            "mcp",
            "add",
            "--transport",
            "stdio",
            "--scope",
            "user",
            "ctxhelpr",
            "--",
            &bin,
            "serve",
        ])
        .status();

    match status {
        Ok(s) if s.success() => println!("done"),
        Ok(s) => {
            println!("warning (exit code {})", s.code().unwrap_or(-1));
            println!("    You may need to register manually:");
            println!(
                "    claude mcp add --transport stdio --scope user ctxhelpr -- {} serve",
                bin
            );
        }
        Err(e) => {
            println!("skipped ({})", e);
            println!("    Claude CLI not found. Register manually:");
            println!(
                "    claude mcp add --transport stdio --scope user ctxhelpr -- {} serve",
                bin
            );
        }
    }

    // 2. Install skill
    let skill_dir = claude_dir()?.join("skills").join("ctxhelpr");
    fs::create_dir_all(&skill_dir)?;
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_CONTENT)?;
    println!("  Installed skill at {}", skill_path.display());

    // 3. Install slash command
    let commands_dir = claude_dir()?.join("commands");
    fs::create_dir_all(&commands_dir)?;
    let cmd_path = commands_dir.join("index.md");
    fs::write(&cmd_path, INDEX_COMMAND_CONTENT)?;
    println!("  Installed /index command at {}", cmd_path.display());

    println!("\nSetup complete! Restart Claude Code to start using ctxhelpr.");
    println!("Try: /index  or ask Claude to \"index this repository\"");

    Ok(())
}
