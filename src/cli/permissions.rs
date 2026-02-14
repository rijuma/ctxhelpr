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
    apply_grants(&mut settings, grants);
    write_settings(path, &settings)
}

pub fn grant_all(path: &Path) -> Result<()> {
    set_grants(path, &[true; 11])
}

pub fn revoke_all(path: &Path) -> Result<()> {
    set_grants(path, &[false; 11])
}

fn apply_grants(settings: &mut Value, grants: &[bool]) {
    let permissions = settings
        .as_object_mut()
        .expect("settings must be an object")
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));

    let allow = permissions
        .as_object_mut()
        .expect("permissions must be an object")
        .entry("allow")
        .or_insert_with(|| serde_json::json!([]));

    let arr = allow.as_array_mut().expect("allow must be an array");

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grant_all_to_empty_settings() {
        let mut settings = json!({});
        apply_grants(&mut settings, &[true; 11]);

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
        apply_grants(&mut settings, &[true; 11]);

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
        apply_grants(&mut settings, &[false; 11]);

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

        apply_grants(&mut settings, &grants);

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
        apply_grants(&mut settings, &[true; 11]);
        apply_grants(&mut settings, &[true; 11]);

        let allow = settings["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 11);
    }
}
