use anyhow::Result;
use std::fs;
use std::process::Command;

use super::{Scope, permissions, resolve_scope, style};
use crate::storage;

pub fn run(scope: Scope) -> Result<()> {
    run_internal(scope)?;

    println!(
        "\n{}",
        style::success("Disable complete. Restart Claude Code to apply changes.")
    );

    Ok(())
}

pub fn run_internal(scope: Scope) -> Result<()> {
    let is_local = matches!(scope, Scope::Local);
    let is_global = matches!(scope, Scope::Global);
    let (base_dir, scope_label) = resolve_scope(scope)?;

    println!(
        "\n{}\n",
        style::heading(&format!("Disabling ctxhelpr ({scope_label})..."))
    );

    // 1. Remove MCP server registration
    print!("  Removing MCP server... ");
    let status = Command::new("claude")
        .args(["mcp", "remove", "ctxhelpr"])
        .status();

    match status {
        Ok(s) if s.success() => println!("{}", style::done()),
        Ok(_) => println!("{}", style::info("not found (may already be removed)")),
        Err(_) => println!("{}", style::info("skipped (claude CLI not found)")),
    }

    // 2. Remove skill
    let skill_dir = base_dir.join("skills").join("ctxhelpr");
    if skill_dir.exists() {
        fs::remove_dir_all(&skill_dir)?;
        println!("  {}", style::info("Removed skill directory"));
    }

    // 3. Remove slash commands (both old /index and new /reindex)
    let old_cmd_path = base_dir.join("commands").join("index.md");
    if old_cmd_path.exists() {
        fs::remove_file(&old_cmd_path)?;
        println!("  {}", style::info("Removed /index command"));
    }
    let reindex_cmd_path = base_dir.join("commands").join("reindex.md");
    if reindex_cmd_path.exists() {
        fs::remove_file(&reindex_cmd_path)?;
        println!("  {}", style::info("Removed /reindex command"));
    }

    // 4. Revoke permissions (non-fatal)
    let settings_path = base_dir.join("settings.json");
    match permissions::revoke_all(&settings_path) {
        Ok(()) => println!("  {}", style::info("Revoked ctxhelpr tool permissions")),
        Err(e) => println!(
            "  {}",
            style::warn(&format!("Could not revoke permissions: {e}"))
        ),
    }

    // 5. Delete index databases
    if is_local {
        let current_dir = std::env::current_dir()?;
        let repo_path = current_dir.to_str().unwrap_or("");
        let db_path = storage::db_path_for_repo(repo_path);
        if db_path.exists() {
            storage::delete_repo_index(repo_path)?;
            println!("  {}", style::info("Deleted index database"));
        }
    } else if is_global {
        let count = storage::delete_all_repo_indexes()?;
        if count > 0 {
            println!(
                "  {}",
                style::info(&format!("Deleted {count} index database(s)"))
            );
        }
    }

    // 6. Delete project configuration
    let config_path = std::env::current_dir()?.join(".ctxhelpr.json");
    if config_path.exists() {
        fs::remove_file(&config_path)?;
        println!("  {}", style::info("Deleted .ctxhelpr.json"));
    }

    Ok(())
}
