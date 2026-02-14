use anyhow::Result;
use dialoguer::Confirm;
use std::fs;
use std::process::Command;

use super::{Scope, permissions, resolve_scope};
use crate::storage;

pub fn run(scope: Scope) -> Result<()> {
    let is_local = matches!(scope, Scope::Local);
    let is_global = matches!(scope, Scope::Global);
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

    // 5. Offer to delete index databases
    if is_local {
        prompt_delete_local_db()?;
    } else if is_global {
        prompt_delete_all_dbs()?;
    }

    println!("\nUninstall complete. Restart Claude Code to apply changes.");

    Ok(())
}

fn prompt_delete_local_db() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo_path = current_dir.to_str().unwrap_or("");
    let db_path = storage::db_path_for_repo(repo_path);
    if !db_path.exists() {
        return Ok(());
    }

    let db_size = fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    let size_str = format_size(db_size);

    println!();
    if Confirm::new()
        .with_prompt(format!(
            "Delete index database for this repository? ({size_str})"
        ))
        .default(true)
        .interact()?
    {
        storage::delete_repo_index(repo_path)?;
        println!("  Deleted index database");
    }
    Ok(())
}

fn prompt_delete_all_dbs() -> Result<()> {
    let repos = storage::list_indexed_repos()?;
    if repos.is_empty() {
        return Ok(());
    }

    let total_size: u64 = repos.iter().map(|r| r.db_size_bytes).sum();
    let size_str = format_size(total_size);

    println!();
    if Confirm::new()
        .with_prompt(format!(
            "Delete all indexed repository databases? ({} repositor{}, {size_str})",
            repos.len(),
            if repos.len() == 1 { "y" } else { "ies" },
        ))
        .default(false)
        .interact()?
    {
        let count = storage::delete_all_repo_indexes()?;
        println!("  Deleted {count} index database(s)");
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
