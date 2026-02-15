use anyhow::Result;
use clap::Subcommand;
use dialoguer::{Confirm, MultiSelect};

use super::style;
use crate::storage;

#[derive(Subcommand)]
pub enum ReposCommands {
    /// List all indexed repositories
    List,
    /// Delete index data for repositories
    Delete {
        /// Repository paths to delete (interactive picker if omitted)
        paths: Vec<String>,
    },
}

pub fn run(command: ReposCommands) -> Result<()> {
    match command {
        ReposCommands::List => list(),
        ReposCommands::Delete { paths } => {
            if paths.is_empty() {
                delete_interactive()
            } else {
                delete_paths(&paths)
            }
        }
    }
}

fn list() -> Result<()> {
    let repos = storage::list_indexed_repos()?;
    if repos.is_empty() {
        println!("No indexed repositories found.");
        return Ok(());
    }

    println!("{}\n", style::heading("Indexed repositories:"));
    let mut total_size: u64 = 0;
    for repo in &repos {
        println!("  {}", repo.abs_path);
        let indexed = repo.last_indexed_at.as_deref().unwrap_or("never");
        println!(
            "    Last indexed: {} | Files: {} | Symbols: {} | DB: {}",
            indexed,
            repo.file_count,
            repo.symbol_count,
            format_size(repo.db_size_bytes),
        );
        println!();
        total_size += repo.db_size_bytes;
    }
    println!(
        "Total: {} repositor{}, {}",
        repos.len(),
        if repos.len() == 1 { "y" } else { "ies" },
        format_size(total_size),
    );
    Ok(())
}

fn delete_interactive() -> Result<()> {
    let repos = storage::list_indexed_repos()?;
    if repos.is_empty() {
        println!("No indexed repositories found.");
        return Ok(());
    }

    let items: Vec<String> = repos
        .iter()
        .map(|r| {
            format!(
                "{} ({}, {} files, {})",
                r.abs_path,
                r.last_indexed_at.as_deref().unwrap_or("never indexed"),
                r.file_count,
                format_size(r.db_size_bytes),
            )
        })
        .collect();

    let selections = MultiSelect::new()
        .with_prompt("Select repositories to delete")
        .items(&items)
        .interact()?;

    if selections.is_empty() {
        println!("No repositories selected.");
        return Ok(());
    }

    let selected_paths: Vec<&str> = selections
        .iter()
        .map(|&i| repos[i].abs_path.as_str())
        .collect();

    println!("\nWill delete index data for:");
    for path in &selected_paths {
        println!("  {path}");
    }

    if !Confirm::new()
        .with_prompt("Proceed?")
        .default(true)
        .interact()?
    {
        println!("{}", style::warn("Cancelled."));
        return Ok(());
    }

    for path in &selected_paths {
        match storage::delete_repo_index(path) {
            Ok(()) => println!("  {}", style::success(&format!("Deleted: {path}"))),
            Err(e) => println!(
                "  {}",
                style::error(&format!("Failed to delete {path}: {e}"))
            ),
        }
    }
    Ok(())
}

fn delete_paths(paths: &[String]) -> Result<()> {
    println!("Will delete index data for:");
    for path in paths {
        println!("  {path}");
    }

    if !Confirm::new()
        .with_prompt("Proceed?")
        .default(true)
        .interact()?
    {
        println!("{}", style::warn("Cancelled."));
        return Ok(());
    }

    for path in paths {
        match storage::delete_repo_index(path) {
            Ok(()) => println!("  {}", style::success(&format!("Deleted: {path}"))),
            Err(e) => println!(
                "  {}",
                style::error(&format!("Failed to delete {path}: {e}"))
            ),
        }
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
