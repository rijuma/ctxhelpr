use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CONFIG_FILENAME: &str = ".ctxhelpr.json";
pub const GLOBAL_CONFIG_DIR: &str = "ctxhelpr";
pub const GLOBAL_CONFIG_FILENAME: &str = "config.json";

#[derive(Debug)]
pub enum ConfigError {
    NotFound {
        path: PathBuf,
    },
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound { path } => {
                write!(f, "no config found at {}", path.display())
            }
            ConfigError::InvalidJson { path, source } => {
                write!(f, "invalid JSON in {}: {}", path.display(), source)
            }
            ConfigError::IoError { path, source } => {
                write!(f, "failed to read {}: {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::InvalidJson { source, .. } => Some(source),
            ConfigError::IoError { source, .. } => Some(source),
            ConfigError::NotFound { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub output: OutputConfig,
    pub search: SearchConfig,
    pub indexer: IndexerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputConfig {
    /// Max tokens for responses (None = unlimited)
    pub max_tokens: Option<usize>,
    /// Max signature length before truncation
    pub truncate_signatures: usize,
    /// Max doc comment length in brief views
    pub truncate_doc_comments: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchConfig {
    /// Max search results returned
    pub max_results: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerConfig {
    /// Glob patterns of paths to ignore during indexing
    pub ignore: Vec<String>,
    /// Max file size in bytes (files larger are skipped)
    pub max_file_size: u64,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_tokens: None,
            truncate_signatures: 120,
            truncate_doc_comments: 100,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { max_results: 20 }
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            ignore: vec![],
            max_file_size: 1_048_576, // 1 MiB
        }
    }
}

/// Returns the path to the global config file, if the platform config dir exists.
pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(GLOBAL_CONFIG_DIR).join(GLOBAL_CONFIG_FILENAME))
}

/// Reads a JSON file as a `serde_json::Value`. Returns empty `{}` if the file doesn't exist.
fn load_json_file(path: &Path) -> Result<Value, ConfigError> {
    if !path.exists() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError {
        path: path.to_path_buf(),
        source: e,
    })?;
    serde_json::from_str(&content).map_err(|e| ConfigError::InvalidJson {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Recursively merges two JSON values. Objects merge key-by-key; arrays and scalars
/// in `overlay` replace whatever is in `base`.
pub fn deep_merge(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let merged = match base_map.remove(&key) {
                    Some(base_val) => deep_merge(base_val, overlay_val),
                    None => overlay_val,
                };
                base_map.insert(key, merged);
            }
            Value::Object(base_map)
        }
        (_, overlay) => overlay,
    }
}

/// Loads and merges global + local config files, then deserializes into `Config`.
/// Global config errors are logged and skipped; local config errors propagate.
pub fn load_and_merge(
    global_path: Option<&Path>,
    local_path: &Path,
) -> Result<Config, ConfigError> {
    let global_value = match global_path {
        Some(path) => match load_json_file(path) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load global config, skipping");
                Value::Object(serde_json::Map::new())
            }
        },
        None => Value::Object(serde_json::Map::new()),
    };

    let local_value = load_json_file(local_path)?;
    let merged = deep_merge(global_value, local_value);

    serde_json::from_value(merged).map_err(|e| ConfigError::InvalidJson {
        path: local_path.to_path_buf(),
        source: e,
    })
}

impl Config {
    pub fn load(repo_path: &str) -> Result<Self, ConfigError> {
        let local_path = Path::new(repo_path).join(CONFIG_FILENAME);
        load_and_merge(global_config_path().as_deref(), &local_path)
    }

    pub fn validate_file(path: &Path) -> Result<Config, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound {
                path: path.to_path_buf(),
            });
        }
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_str(&content).map_err(|e| ConfigError::InvalidJson {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn validate(repo_path: &str) -> Result<Config, ConfigError> {
        let config_path = Path::new(repo_path).join(CONFIG_FILENAME);
        Self::validate_file(&config_path)
    }

    pub fn validate_global() -> Result<Config, ConfigError> {
        let path = global_config_path().ok_or_else(|| ConfigError::NotFound {
            path: PathBuf::from("<no config dir>"),
        })?;
        Self::validate_file(&path)
    }
}

/// Thread-safe cache for per-repo configs.
pub struct ConfigCache {
    cache: Mutex<HashMap<String, Config>>,
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, repo_path: &str) -> Config {
        let mut cache = self.cache.lock().unwrap();
        if let Some(config) = cache.get(repo_path) {
            return config.clone();
        }
        let config = match Config::load(repo_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(path = %repo_path, error = %e, "Failed to load config, using defaults");
                Config::default()
            }
        };
        cache.insert(repo_path.to_string(), config.clone());
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.search.max_results, 20);
        assert_eq!(config.indexer.max_file_size, 1_048_576);
        assert_eq!(config.output.truncate_signatures, 120);
    }

    #[test]
    fn test_load_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let local_path = dir.path().join(CONFIG_FILENAME);
        let config = load_and_merge(None, &local_path).unwrap();
        assert_eq!(config.search.max_results, 20);
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_content = r#"{
  "output": {
    "max_tokens": 2000,
    "truncate_signatures": 80
  },
  "search": {
    "max_results": 10
  },
  "indexer": {
    "ignore": ["generated/", "*.min.js"],
    "max_file_size": 524288
  }
}"#;
        fs::write(dir.path().join(CONFIG_FILENAME), config_content).unwrap();

        let config = Config::load(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(config.output.max_tokens, Some(2000));
        assert_eq!(config.output.truncate_signatures, 80);
        assert_eq!(config.search.max_results, 10);
        assert_eq!(config.indexer.ignore, vec!["generated/", "*.min.js"]);
        assert_eq!(config.indexer.max_file_size, 524288);
    }

    #[test]
    fn test_partial_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_content = r#"{ "search": { "max_results": 5 } }"#;
        fs::write(dir.path().join(CONFIG_FILENAME), config_content).unwrap();

        let config = Config::load(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(config.search.max_results, 5);
        assert_eq!(config.output.truncate_signatures, 120);
        assert_eq!(config.indexer.max_file_size, 1_048_576);
    }

    #[test]
    fn test_config_cache() {
        let cache = ConfigCache::new();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let c1 = cache.get(path);
        let c2 = cache.get(path);
        assert_eq!(c1.search.max_results, c2.search.max_results);
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILENAME), "{bad json").unwrap();

        let result = Config::load(dir.path().to_str().unwrap());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidJson { .. }));
    }

    #[test]
    fn test_load_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config_content = r#"{ "search": { "max_results": 10, "typo_field": true } }"#;
        fs::write(dir.path().join(CONFIG_FILENAME), config_content).unwrap();

        let result = Config::load(dir.path().to_str().unwrap());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidJson { .. }));
        assert!(err.to_string().contains("typo_field"));
    }

    #[test]
    fn test_validate_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = Config::validate(dir.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::NotFound { .. }));
    }

    #[test]
    fn test_validate_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_content = r#"{ "search": { "max_results": 10 } }"#;
        fs::write(dir.path().join(CONFIG_FILENAME), config_content).unwrap();

        let config = Config::validate(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(config.search.max_results, 10);
    }

    // --- deep_merge tests ---

    #[test]
    fn test_deep_merge_disjoint_keys() {
        let base: Value = serde_json::json!({"a": 1});
        let overlay: Value = serde_json::json!({"b": 2});
        let merged = deep_merge(base, overlay);
        assert_eq!(merged, serde_json::json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_deep_merge_overlapping_keys() {
        let base: Value = serde_json::json!({"a": 1, "b": 2});
        let overlay: Value = serde_json::json!({"b": 99});
        let merged = deep_merge(base, overlay);
        assert_eq!(merged, serde_json::json!({"a": 1, "b": 99}));
    }

    #[test]
    fn test_deep_merge_nested_partial() {
        let base: Value = serde_json::json!({"search": {"max_results": 50}});
        let overlay: Value = serde_json::json!({"indexer": {"ignore": ["dist/"]}});
        let merged = deep_merge(base, overlay);
        assert_eq!(
            merged,
            serde_json::json!({"search": {"max_results": 50}, "indexer": {"ignore": ["dist/"]}})
        );
    }

    #[test]
    fn test_deep_merge_nested_override() {
        let base: Value =
            serde_json::json!({"output": {"max_tokens": 1000, "truncate_signatures": 80}});
        let overlay: Value = serde_json::json!({"output": {"max_tokens": 2000}});
        let merged = deep_merge(base, overlay);
        assert_eq!(
            merged,
            serde_json::json!({"output": {"max_tokens": 2000, "truncate_signatures": 80}})
        );
    }

    #[test]
    fn test_deep_merge_array_replacement() {
        let base: Value = serde_json::json!({"indexer": {"ignore": ["node_modules/"]}});
        let overlay: Value = serde_json::json!({"indexer": {"ignore": ["dist/", "build/"]}});
        let merged = deep_merge(base, overlay);
        assert_eq!(
            merged,
            serde_json::json!({"indexer": {"ignore": ["dist/", "build/"]}})
        );
    }

    #[test]
    fn test_deep_merge_empty_base() {
        let base: Value = serde_json::json!({});
        let overlay: Value = serde_json::json!({"search": {"max_results": 10}});
        let merged = deep_merge(base, overlay);
        assert_eq!(merged, serde_json::json!({"search": {"max_results": 10}}));
    }

    #[test]
    fn test_deep_merge_empty_overlay() {
        let base: Value = serde_json::json!({"search": {"max_results": 50}});
        let overlay: Value = serde_json::json!({});
        let merged = deep_merge(base, overlay);
        assert_eq!(merged, serde_json::json!({"search": {"max_results": 50}}));
    }

    // --- load_and_merge tests ---

    #[test]
    fn test_load_and_merge_both_present() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("local.json");

        fs::write(&global_path, r#"{"search": {"max_results": 50}}"#).unwrap();
        fs::write(&local_path, r#"{"indexer": {"ignore": ["dist/"]}}"#).unwrap();

        let config = load_and_merge(Some(&global_path), &local_path).unwrap();
        assert_eq!(config.search.max_results, 50);
        assert_eq!(config.indexer.ignore, vec!["dist/"]);
        // Defaults fill the rest
        assert_eq!(config.output.truncate_signatures, 120);
    }

    #[test]
    fn test_load_and_merge_local_overrides_global() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("local.json");

        fs::write(&global_path, r#"{"search": {"max_results": 50}}"#).unwrap();
        fs::write(&local_path, r#"{"search": {"max_results": 10}}"#).unwrap();

        let config = load_and_merge(Some(&global_path), &local_path).unwrap();
        assert_eq!(config.search.max_results, 10);
    }

    #[test]
    fn test_load_and_merge_global_only() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("nonexistent.json");

        fs::write(&global_path, r#"{"search": {"max_results": 50}}"#).unwrap();

        let config = load_and_merge(Some(&global_path), &local_path).unwrap();
        assert_eq!(config.search.max_results, 50);
    }

    #[test]
    fn test_load_and_merge_local_only() {
        let dir = tempfile::tempdir().unwrap();
        let local_path = dir.path().join("local.json");

        fs::write(&local_path, r#"{"search": {"max_results": 10}}"#).unwrap();

        let config = load_and_merge(None, &local_path).unwrap();
        assert_eq!(config.search.max_results, 10);
    }

    #[test]
    fn test_load_and_merge_both_missing() {
        let dir = tempfile::tempdir().unwrap();
        let local_path = dir.path().join("nonexistent.json");

        let config = load_and_merge(None, &local_path).unwrap();
        assert_eq!(config.search.max_results, 20); // default
    }

    #[test]
    fn test_load_and_merge_invalid_global_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("local.json");

        fs::write(&global_path, "{bad json").unwrap();
        fs::write(&local_path, r#"{"search": {"max_results": 10}}"#).unwrap();

        let config = load_and_merge(Some(&global_path), &local_path).unwrap();
        assert_eq!(config.search.max_results, 10);
    }

    #[test]
    fn test_load_and_merge_invalid_local_errors() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("local.json");

        fs::write(&global_path, r#"{"search": {"max_results": 50}}"#).unwrap();
        fs::write(&local_path, "{bad json").unwrap();

        let result = load_and_merge(Some(&global_path), &local_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidJson { .. }
        ));
    }

    #[test]
    fn test_load_and_merge_unknown_field_detected() {
        let dir = tempfile::tempdir().unwrap();
        let global_path = dir.path().join("global.json");
        let local_path = dir.path().join("local.json");

        fs::write(&global_path, r#"{"typo_field": true}"#).unwrap();
        fs::write(&local_path, r#"{}"#).unwrap();

        // Invalid global is skipped (warn + fallback), but the merged value
        // is deserialized with deny_unknown_fields — however since global is
        // skipped on parse error, this particular case: global has valid JSON
        // but unknown field. The JSON itself parses fine, the error comes at
        // deserialization of the merged value.
        // Actually, global JSON parses fine as Value (typo_field is valid JSON).
        // The error surfaces when deserializing into Config.
        let result = load_and_merge(Some(&global_path), &local_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("typo_field"));
    }

    #[test]
    fn test_error_display_includes_path() {
        let path = PathBuf::from("/some/config.json");
        let err = ConfigError::NotFound { path: path.clone() };
        assert!(err.to_string().contains("/some/config.json"));

        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = ConfigError::IoError {
            path,
            source: io_err,
        };
        assert!(err.to_string().contains("/some/config.json"));
        assert!(err.to_string().contains("denied"));
    }
}
