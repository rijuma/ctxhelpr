use anyhow::Result;
use dialoguer::Confirm;
use std::fs;

use super::Scope;

pub fn run() -> Result<()> {
    if !Confirm::new()
        .with_prompt(
            "Completely uninstall ctxhelpr? This will disable all integrations and remove the binary",
        )
        .default(false)
        .interact()?
    {
        println!("Cancelled.");
        return Ok(());
    }

    let global_settings = super::claude_dir()?.join("settings.json");
    if global_settings.exists() {
        super::disable::run_internal(Scope::Global)?;
    }

    let local_settings = super::project_claude_dir()?.join("settings.json");
    if local_settings.exists() {
        super::disable::run_internal(Scope::Local)?;
    }

    let exe_path = std::env::current_exe()?;
    fs::remove_file(&exe_path)?;
    println!("\n  Removed binary: {}", exe_path.display());

    println!("\nctxhelpr has been uninstalled.");

    Ok(())
}
