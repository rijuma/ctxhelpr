pub mod config_cmd;
pub mod install;
pub mod permissions;
pub mod perms;
pub mod repos;
pub mod uninstall;

use anyhow::Result;
use std::path::PathBuf;

pub enum Scope {
    Local,
    Global,
    Unspecified,
}

pub fn resolve_scope(scope: Scope) -> Result<(PathBuf, &'static str)> {
    match scope {
        Scope::Local => Ok((project_claude_dir()?, "local")),
        Scope::Global => Ok((claude_dir()?, "global")),
        Scope::Unspecified => {
            let local_settings = project_claude_dir()?.join("settings.json");
            if local_settings.exists() {
                Ok((project_claude_dir()?, "local"))
            } else {
                Ok((claude_dir()?, "global"))
            }
        }
    }
}

pub fn claude_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    Ok(home.join(".claude"))
}

pub fn project_claude_dir() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(".claude"))
}
