use anyhow::Result;
use std::fs;
use std::process::Command;

use super::{Scope, permissions, style};
use crate::storage::db_path_for_repo;

const SKILL_CONTENT: &str = include_str!("../assets/skill.md");
const REINDEX_COMMAND_CONTENT: &str = include_str!("../assets/reindex_command.md");

fn binary_path() -> Result<String> {
    std::env::current_exe()?
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Binary path is not valid UTF-8"))
}

fn prompt_scope() -> Result<Scope> {
    let options = &["Local (this project)", "Global (~/.claude/)"];
    let selection = dialoguer::Select::new()
        .with_prompt("Where do you want to enable ctxhelpr?")
        .items(options)
        .default(0)
        .interact()
        .map_err(|_| anyhow::anyhow!("Enable cancelled."))?;

    match selection {
        0 => Ok(Scope::Local),
        _ => Ok(Scope::Global),
    }
}

pub fn run(scope: Scope) -> Result<()> {
    let scope = match scope {
        Scope::Unspecified => prompt_scope()?,
        other => other,
    };

    let (base_dir, scope_label) = super::resolve_scope(scope)?;

    let mcp_scope = match scope_label {
        "local" => "project",
        _ => "user",
    };

    let cwd = std::env::current_dir()?;
    match &scope {
        Scope::Global => println!("{}\n", style::heading("Enabling ctxhelpr globally...")),
        _ => println!(
            "{}\n",
            style::heading(&format!(
                "Enabling ctxhelpr locally for {}...",
                cwd.display()
            ))
        ),
    }

    // 1. Register MCP server
    let bin = binary_path()?;
    print!("  Registering MCP server (scope: {mcp_scope})... ");
    let status = Command::new("claude")
        .args([
            "mcp",
            "add",
            "--transport",
            "stdio",
            "--scope",
            mcp_scope,
            "ctxhelpr",
            "--",
            &bin,
            "serve",
        ])
        .status();

    match status {
        Ok(s) if s.success() => println!("{}", style::done()),
        Ok(s) => {
            println!(
                "{}",
                style::warn(&format!("warning (exit code {})", s.code().unwrap_or(-1)))
            );
            println!("    You may need to register manually:");
            println!(
                "    claude mcp add --transport stdio --scope {mcp_scope} ctxhelpr -- {} serve",
                bin
            );
        }
        Err(e) => {
            println!("{}", style::warn(&format!("skipped ({e})")));
            println!("    Claude CLI not found. Register manually:");
            println!(
                "    claude mcp add --transport stdio --scope {mcp_scope} ctxhelpr -- {} serve",
                bin
            );
        }
    }

    // 2. Install skill
    let skill_dir = base_dir.join("skills").join("ctxhelpr");
    fs::create_dir_all(&skill_dir)?;
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_CONTENT)?;
    println!(
        "  Installed skill at {}",
        style::info(&skill_path.display().to_string())
    );

    // 3. Install slash command (reindex, replacing old index)
    let commands_dir = base_dir.join("commands");
    fs::create_dir_all(&commands_dir)?;

    // Clean up old /index command if present
    let old_cmd_path = commands_dir.join("index.md");
    if old_cmd_path.exists() {
        let _ = fs::remove_file(&old_cmd_path);
    }

    let cmd_path = commands_dir.join("reindex.md");
    fs::write(&cmd_path, REINDEX_COMMAND_CONTENT)?;
    println!(
        "  Installed /reindex command at {}",
        style::info(&cmd_path.display().to_string())
    );

    // 4. Permission prompt
    let settings_path = base_dir.join("settings.json");
    let grant = dialoguer::Confirm::new()
        .with_prompt("Grant all ctxhelpr tool permissions? (avoids prompts during use)")
        .default(true)
        .interact()
        .unwrap_or(false);

    if grant {
        permissions::grant_all(&settings_path)?;
        println!(
            "  {}",
            style::success(&format!("Granted all tool permissions ({scope_label})"))
        );
    } else {
        println!("  Skipped permissions. Run `ctxhelpr perms` to configure later.");
    }

    // 5. Print DB path
    let abs_path = cwd
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Current directory is not valid UTF-8"))?;
    let db_path = db_path_for_repo(abs_path);
    println!(
        "\n  Index database: {}",
        style::info(&db_path.display().to_string())
    );

    println!(
        "\n{}",
        style::success("Enable complete! Restart Claude Code to start using ctxhelpr.")
    );
    println!("Repos are auto-indexed on startup. Use /reindex to force a refresh.");

    Ok(())
}
