use anyhow::Result;
use std::fs;
use std::process::Command;

use super::{Scope, permissions, resolve_scope};

pub fn run(scope: Scope) -> Result<()> {
    let (base_dir, scope_label) = resolve_scope(scope)?;

    println!("Removing ctxhelpr ({scope_label})...\n");

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
    let skill_dir = base_dir.join("skills").join("ctxhelpr");
    if skill_dir.exists() {
        fs::remove_dir_all(&skill_dir)?;
        println!("  Removed skill directory");
    }

    // 3. Remove slash command
    let cmd_path = base_dir.join("commands").join("index.md");
    if cmd_path.exists() {
        fs::remove_file(&cmd_path)?;
        println!("  Removed /index command");
    }

    // 4. Revoke permissions (non-fatal)
    let settings_path = base_dir.join("settings.json");
    match permissions::revoke_all(&settings_path) {
        Ok(()) => println!("  Revoked ctxhelpr tool permissions"),
        Err(e) => println!("  Could not revoke permissions: {e}"),
    }

    println!("\nUninstall complete. Restart Claude Code to apply changes.");

    Ok(())
}
