use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

pub const CONFIG_FILENAME: &str = ".ctxhelpr.json";

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    InvalidJson { source: serde_json::Error },
    IoError { source: std::io::Error },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "no {} found", CONFIG_FILENAME),
            ConfigError::InvalidJson { source } => {
                write!(f, "invalid JSON in {}: {}", CONFIG_FILENAME, source)
            }
            ConfigError::IoError { source } => {
                write!(f, "failed to read {}: {}", CONFIG_FILENAME, source)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::InvalidJson { source } => Some(source),
            ConfigError::IoError { source } => Some(source),
            ConfigError::NotFound => None,
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

impl Config {
    pub fn load(repo_path: &str) -> Result<Self, ConfigError> {
        let config_path = Path::new(repo_path).join(CONFIG_FILENAME);
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::IoError { source: e })?;
        let config: Config =
            serde_json::from_str(&content).map_err(|e| ConfigError::InvalidJson { source: e })?;
        Ok(config)
    }

    pub fn validate(repo_path: &str) -> Result<Config, ConfigError> {
        let config_path = Path::new(repo_path).join(CONFIG_FILENAME);
        if !config_path.exists() {
            return Err(ConfigError::NotFound);
        }
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::IoError { source: e })?;
        let config: Config =
            serde_json::from_str(&content).map_err(|e| ConfigError::InvalidJson { source: e })?;
        Ok(config)
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
                tracing::warn!(path = %repo_path, error = %e, "Failed to load .ctxhelpr.json, using defaults");
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
        let config = Config::load(dir.path().to_str().unwrap()).unwrap();
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
        // Unspecified values use defaults
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
        assert!(matches!(result.unwrap_err(), ConfigError::NotFound));
    }

    #[test]
    fn test_validate_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_content = r#"{ "search": { "max_results": 10 } }"#;
        fs::write(dir.path().join(CONFIG_FILENAME), config_content).unwrap();

        let config = Config::validate(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(config.search.max_results, 10);
    }
}
