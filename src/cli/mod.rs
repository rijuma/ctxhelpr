pub mod setup;
pub mod uninstall;

use anyhow::Result;
use std::path::PathBuf;

pub fn claude_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home.join(".claude"))
}
