use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub const TOOL_PERMISSIONS: [&str; 11] = [
    "mcp__ctxhelpr__index_repository",
    "mcp__ctxhelpr__update_files",
    "mcp__ctxhelpr__get_overview",
    "mcp__ctxhelpr__get_file_symbols",
    "mcp__ctxhelpr__get_symbol_detail",
    "mcp__ctxhelpr__search_symbols",
    "mcp__ctxhelpr__get_references",
    "mcp__ctxhelpr__get_dependencies",
    "mcp__ctxhelpr__index_status",
    "mcp__ctxhelpr__list_repos",
    "mcp__ctxhelpr__delete_repos",
];

pub const TOOL_LABELS: [&str; 11] = [
    "index_repository  - Full index/re-index",
    "update_files      - Re-index specific files after edits",
    "get_overview      - High-level repo structure",
    "get_file_symbols  - All symbols in a file",
    "get_symbol_detail - Full symbol details",
    "search_symbols    - Full-text search",
    "get_references    - Who references a symbol",
    "get_dependencies  - What a symbol depends on",
    "index_status      - Check index freshness",
    "list_repos        - List all indexed repositories",
    "delete_repos      - Delete repository index data",
];

pub fn read_settings(path: &Path) -> Result<Value> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(serde_json::json!({}))
    }
}

pub fn write_settings(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn current_grants(path: &Path) -> Result<Vec<bool>> {
    let settings = read_settings(path)?;
    let allowed = settings
        .get("permissions")
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array());

    Ok(TOOL_PERMISSIONS
        .iter()
        .map(|perm| {
            allowed
                .map(|arr| arr.iter().any(|v| v.as_str() == Some(perm)))
                .unwrap_or(false)
        })
        .collect())
}

pub fn set_grants(path: &Path, grants: &[bool]) -> Result<()> {
    let mut settings = read_settings(path)?;
    apply_grants(&mut settings, grants)?;
    write_settings(path, &settings)
}

pub fn grant_all(path: &Path) -> Result<()> {
    set_grants(path, &[true; 11])
}

pub fn revoke_all(path: &Path) -> Result<()> {
    set_grants(path, &[false; 11])
}

fn apply_grants(settings: &mut Value, grants: &[bool]) -> Result<()> {
    let permissions = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("settings is not a JSON object"))?
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));

    let allow = permissions
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("permissions is not a JSON object"))?
        .entry("allow")
        .or_insert_with(|| serde_json::json!([]));

    let arr = allow
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("allow is not a JSON array"))?;

    // Remove existing ctxhelpr entries
    arr.retain(|v| {
        v.as_str()
            .map(|s| !s.starts_with("mcp__ctxhelpr__"))
            .unwrap_or(true)
    });

    // Add granted permissions
    for (i, granted) in grants.iter().enumerate() {
        if *granted {
            arr.push(Value::String(TOOL_PERMISSIONS[i].to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grant_all_to_empty_settings() {
        let mut settings = json!({});
        apply_grants(&mut settings, &[true; 11]).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 11);
        for perm in &TOOL_PERMISSIONS {
            assert!(allow.contains(&json!(perm)));
        }
    }

    #[test]
    fn grant_all_preserves_non_ctxhelpr_entries() {
        let mut settings = json!({
            "permissions": {
                "allow": ["mcp__other__tool", "some_permission"],
                "deny": ["something"]
            },
            "other_key": true
        });
        apply_grants(&mut settings, &[true; 11]).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 13); // 2 existing + 11 ctxhelpr
        assert!(allow.contains(&json!("mcp__other__tool")));
        assert!(allow.contains(&json!("some_permission")));
        assert_eq!(settings["permissions"]["deny"][0], "something");
        assert_eq!(settings["other_key"], true);
    }

    #[test]
    fn revoke_all_removes_only_ctxhelpr_entries() {
        let mut settings = json!({
            "permissions": {
                "allow": [
                    "mcp__other__tool",
                    "mcp__ctxhelpr__index_repository",
                    "mcp__ctxhelpr__get_overview",
                    "some_permission"
                ]
            }
        });
        apply_grants(&mut settings, &[false; 11]).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 2);
        assert!(allow.contains(&json!("mcp__other__tool")));
        assert!(allow.contains(&json!("some_permission")));
    }

    #[test]
    fn selective_grants() {
        let mut settings = json!({});
        let mut grants = [false; 11];
        grants[0] = true; // index_repository
        grants[2] = true; // get_overview
        grants[5] = true; // search_symbols
        grants[7] = true; // get_dependencies
        grants[8] = true; // index_status

        apply_grants(&mut settings, &grants).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 5);
        assert!(allow.contains(&json!("mcp__ctxhelpr__index_repository")));
        assert!(allow.contains(&json!("mcp__ctxhelpr__get_overview")));
        assert!(allow.contains(&json!("mcp__ctxhelpr__search_symbols")));
        assert!(allow.contains(&json!("mcp__ctxhelpr__get_dependencies")));
        assert!(allow.contains(&json!("mcp__ctxhelpr__index_status")));
    }

    #[test]
    fn idempotent_grant_no_duplicates() {
        let mut settings = json!({});
        apply_grants(&mut settings, &[true; 11]).unwrap();
        apply_grants(&mut settings, &[true; 11]).unwrap();

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 11);
    }

    #[test]
    fn apply_grants_rejects_non_object_settings() {
        let mut settings = json!("not an object");
        let result = apply_grants(&mut settings, &[true; 11]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a JSON object")
        );
    }

    #[test]
    fn apply_grants_rejects_non_object_permissions() {
        let mut settings = json!({"permissions": "not an object"});
        let result = apply_grants(&mut settings, &[true; 11]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a JSON object")
        );
    }

    #[test]
    fn apply_grants_rejects_non_array_allow() {
        let mut settings = json!({"permissions": {"allow": "not an array"}});
        let result = apply_grants(&mut settings, &[true; 11]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a JSON array"));
    }

    #[test]
    fn round_trip_grant_all_then_verify() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        grant_all(&path).unwrap();
        let grants = current_grants(&path).unwrap();
        assert!(grants.iter().all(|&g| g), "All grants should be true");

        revoke_all(&path).unwrap();
        let grants = current_grants(&path).unwrap();
        assert!(grants.iter().all(|&g| !g), "All grants should be false");
    }

    #[test]
    fn current_grants_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");

        let grants = current_grants(&path).unwrap();
        assert!(
            grants.iter().all(|&g| !g),
            "Missing file should yield all false"
        );
    }

    #[test]
    fn set_grants_preserves_non_ctxhelpr_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        // Write settings with non-ctxhelpr permissions
        let initial = json!({
            "permissions": {
                "allow": ["other_tool_permission"],
                "deny": ["something_else"]
            },
            "unrelated_key": 42
        });
        write_settings(&path, &initial).unwrap();

        set_grants(&path, &[true; 11]).unwrap();

        let settings = read_settings(&path).unwrap();
        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert!(allow.contains(&json!("other_tool_permission")));
        assert_eq!(settings["permissions"]["deny"][0], "something_else");
        assert_eq!(settings["unrelated_key"], 42);
        assert_eq!(allow.len(), 12); // 1 existing + 11 ctxhelpr
    }
}
