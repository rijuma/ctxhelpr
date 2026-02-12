use anyhow::Result;

use super::{Scope, permissions, resolve_scope};

pub fn run(scope: Scope, all: bool, remove: bool) -> Result<()> {
    let (base_dir, scope_label) = resolve_scope(scope)?;
    let settings_path = base_dir.join("settings.json");

    if all {
        permissions::grant_all(&settings_path)?;
        println!("Granted all ctxhelpr tool permissions ({scope_label}).");
        println!("  Settings: {}", settings_path.display());
        return Ok(());
    }

    if remove {
        permissions::revoke_all(&settings_path)?;
        println!("Revoked all ctxhelpr tool permissions ({scope_label}).");
        println!("  Settings: {}", settings_path.display());
        return Ok(());
    }

    // Interactive mode
    let current = permissions::current_grants(&settings_path)?;
    let defaults: Vec<bool> = current.clone();

    let selections = match dialoguer::MultiSelect::new()
        .with_prompt("Select ctxhelpr tools to allow (space to toggle, enter to confirm)")
        .items(permissions::TOOL_LABELS)
        .defaults(&defaults)
        .interact()
    {
        Ok(s) => s,
        Err(_) => {
            println!("Cancelled. Permissions unchanged.");
            return Ok(());
        }
    };

    let mut grants = [false; 9];
    for idx in &selections {
        grants[*idx] = true;
    }

    permissions::set_grants(&settings_path, &grants)?;

    let granted_count = selections.len();
    println!("Updated ctxhelpr permissions: {granted_count}/9 tools allowed ({scope_label}).");
    println!("  Settings: {}", settings_path.display());

    Ok(())
}
