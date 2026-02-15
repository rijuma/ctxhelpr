use anyhow::Result;
use std::fs;

use super::{Scope, style};
use crate::storage;

pub fn run() -> Result<()> {
    let global_settings = super::claude_dir()?.join("settings.json");
    if global_settings.exists() {
        super::disable::run_internal(Scope::Global)?;
    }

    let local_settings = super::project_claude_dir()?.join("settings.json");
    if local_settings.exists() {
        super::disable::run_internal(Scope::Local)?;
    }

    // Remove entire cache directory (all index data)
    storage::delete_cache_dir()?;
    println!("  {}", style::info("Deleted cache directory"));

    let exe_path = std::env::current_exe()?;
    fs::remove_file(&exe_path)?;
    println!(
        "\n  {}",
        style::info(&format!("Removed binary: {}", exe_path.display()))
    );

    println!("\n{}", style::success("ctxhelpr has been uninstalled."));

    Ok(())
}
