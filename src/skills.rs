use std::path::PathBuf;

pub const SKILL_CONTENT: &str = include_str!("assets/skill.md");
pub const REINDEX_COMMAND_CONTENT: &str = include_str!("assets/reindex_command.md");

/// Returns the base dirs to check for a given repo path: `[~/.claude, {repo}/.claude]`.
pub fn base_dirs_for_repo(repo_path: &str) -> Vec<PathBuf> {
    [
        dirs::home_dir().map(|h| h.join(".claude")),
        Some(std::path::Path::new(repo_path).join(".claude")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Returns the base dirs to check for the current working directory: `[~/.claude, {cwd}/.claude]`.
pub fn base_dirs_for_cwd() -> Vec<PathBuf> {
    [
        dirs::home_dir().map(|h| h.join(".claude")),
        std::env::current_dir().ok().map(|d| d.join(".claude")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Overwrites existing skill/command files with embedded content, cleans up old `/index` command.
/// Returns the number of items refreshed.
pub fn refresh(base_dirs: &[PathBuf]) -> usize {
    let mut count = 0;

    for base in base_dirs {
        let skill_path = base.join("skills").join("ctxhelpr").join("SKILL.md");
        if skill_path.exists() {
            if let Err(e) = std::fs::write(&skill_path, SKILL_CONTENT) {
                tracing::debug!(path = %skill_path.display(), error = %e, "failed to refresh skill file");
            } else {
                tracing::debug!(path = %skill_path.display(), "refreshed skill file");
                count += 1;
            }
        }

        let reindex_path = base.join("commands").join("reindex.md");
        if reindex_path.exists() {
            if let Err(e) = std::fs::write(&reindex_path, REINDEX_COMMAND_CONTENT) {
                tracing::debug!(path = %reindex_path.display(), error = %e, "failed to refresh command file");
            } else {
                tracing::debug!(path = %reindex_path.display(), "refreshed command file");
                count += 1;
            }
        }

        // Clean up old /index command if present
        let old_cmd_path = base.join("commands").join("index.md");
        if old_cmd_path.exists() {
            let _ = std::fs::remove_file(&old_cmd_path);
            // Install reindex.md in its place
            if !reindex_path.exists() {
                let _ = std::fs::write(&reindex_path, REINDEX_COMMAND_CONTENT);
            }
            count += 1;
        }
    }

    count
}
