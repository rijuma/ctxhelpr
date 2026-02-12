use anyhow::Result;
use std::fs;
use std::process::Command;

use super::claude_dir;

pub fn run() -> Result<()> {
    println!("Removing ctxhelpr from Claude Code...\n");

    // 1. Remove MCP server registration
    print!("  Removing MCP server... ");
    let status = Command::new("claude")
        .args(["mcp", "remove", "ctxhelpr"])
        .status();

    match status {
        Ok(s) if s.success() => println!("done"),
        Ok(_) => println!("not found (may already be removed)"),
        Err(_) => println!("skipped (claude CLI not found)"),
    }

    // 2. Remove skill
    let skill_dir = claude_dir()?.join("skills").join("ctxhelpr");
    if skill_dir.exists() {
        fs::remove_dir_all(&skill_dir)?;
        println!("  Removed skill directory");
    }

    // 3. Remove slash command
    let cmd_path = claude_dir()?.join("commands").join("index.md");
    if cmd_path.exists() {
        fs::remove_file(&cmd_path)?;
        println!("  Removed /index command");
    }

    println!("\nUninstall complete. Restart Claude Code to apply changes.");

    Ok(())
}
