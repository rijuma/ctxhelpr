pub mod debouncer;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use ignore::gitignore::Gitignore;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::config::ConfigCache;
use crate::indexer::{self, Indexer};
use crate::storage::{self, SqliteStorage};
use debouncer::{Debouncer, FileChangeKind};

/// Commands sent from the MCP server to the watcher event loop.
enum WatcherCommand {
    Watch {
        repo_path: String,
    },
    Unwatch {
        repo_path: String,
    },
    #[allow(dead_code)]
    Shutdown,
}

/// Filesystem event bridged from notify's OS thread to the tokio event loop.
struct FsEvent {
    repo_path: String,
    rel_path: String,
    kind: FileChangeKind,
}

/// Handle for the MCP server to communicate with the watcher.
#[derive(Clone)]
pub struct WatcherHandle {
    cmd_tx: mpsc::Sender<WatcherCommand>,
}

impl WatcherHandle {
    /// Start watching a newly-indexed repo.
    pub async fn watch_repo(&self, path: &str) {
        let _ = self
            .cmd_tx
            .send(WatcherCommand::Watch {
                repo_path: path.to_string(),
            })
            .await;
    }

    /// Stop watching a deleted repo.
    pub async fn unwatch_repo(&self, path: &str) {
        let _ = self
            .cmd_tx
            .send(WatcherCommand::Unwatch {
                repo_path: path.to_string(),
            })
            .await;
    }

    /// Shut down the watcher event loop.
    #[allow(dead_code)]
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(WatcherCommand::Shutdown).await;
    }
}

/// Per-repo watcher state.
struct RepoWatcher {
    /// Dropping this stops the OS-level watcher.
    _watcher: RecommendedWatcher,
    /// Ignore patterns from the repo's config.
    ignore_patterns: Vec<String>,
    /// Max file size from the repo's config.
    max_file_size: u64,
    /// Gitignore matcher for filtering watched files.
    gitignore: Option<Gitignore>,
}

/// Start the watcher subsystem. Reindexes known repos on startup (blocking),
/// then watches for filesystem changes in the background.
pub async fn start(indexer: Arc<Indexer>, config_cache: Arc<ConfigCache>) -> WatcherHandle {
    let (cmd_tx, cmd_rx) = mpsc::channel::<WatcherCommand>(64);
    let (fs_tx, fs_rx) = mpsc::channel::<FsEvent>(512);

    // Discover and reindex known repos (blocking — tools wait for this)
    let repos = storage::list_indexed_repos().unwrap_or_default();
    let mut initial_watchers: HashMap<String, RepoWatcher> = HashMap::new();

    for repo in &repos {
        let repo_path = &repo.abs_path;
        if !Path::new(repo_path).is_dir() {
            tracing::info!(path = %repo_path, "Skipping missing repo directory");
            continue;
        }

        tracing::info!(path = %repo_path, "Reindexing on startup");
        let config = config_cache.get(repo_path);
        let indexer_clone = indexer.clone();
        let path = repo_path.clone();
        let ignore = config.indexer.ignore.clone();
        let max_size = config.indexer.max_file_size;

        // Block on reindex so the index is fresh before any tool responds
        let result = tokio::task::spawn_blocking(move || {
            let storage = SqliteStorage::open(&path)?;
            indexer_clone.index(&path, &storage, &ignore, max_size)
        })
        .await;

        match result {
            Ok(Ok(stats)) => {
                tracing::info!(
                    path = %repo_path,
                    files = stats.files_total,
                    changed = stats.files_changed,
                    duration_ms = stats.duration_ms,
                    "Startup reindex complete"
                );
                crate::skills::refresh(&crate::skills::base_dirs_for_repo(repo_path));
            }
            Ok(Err(e)) => {
                tracing::warn!(path = %repo_path, error = %e, "Startup reindex failed");
                continue;
            }
            Err(e) => {
                tracing::warn!(path = %repo_path, error = %e, "Startup reindex task panicked");
                continue;
            }
        }

        // Start watching this repo
        if let Some(rw) = create_repo_watcher(repo_path, &config_cache, &fs_tx) {
            initial_watchers.insert(repo_path.clone(), rw);
        }
    }

    // Spawn the main event loop
    tokio::spawn(event_loop(
        indexer,
        config_cache,
        cmd_rx,
        fs_tx,
        fs_rx,
        initial_watchers,
    ));

    WatcherHandle { cmd_tx }
}

fn load_gitignore(repo_path: &str) -> Option<Gitignore> {
    let gitignore_path = Path::new(repo_path).join(".gitignore");
    if !gitignore_path.exists() {
        return None;
    }
    let (gi, err) = Gitignore::new(&gitignore_path);
    if let Some(e) = err {
        tracing::debug!(path = %repo_path, error = %e, "Error parsing .gitignore");
    }
    Some(gi)
}

fn create_repo_watcher(
    repo_path: &str,
    config_cache: &ConfigCache,
    fs_tx: &mpsc::Sender<FsEvent>,
) -> Option<RepoWatcher> {
    let config = config_cache.get(repo_path);
    let ignore_patterns = config.indexer.ignore.clone();
    let max_file_size = config.indexer.max_file_size;
    let gitignore = load_gitignore(repo_path);
    let repo_path_owned = repo_path.to_string();
    let repo_path_buf = PathBuf::from(repo_path);
    let callback_gitignore = load_gitignore(repo_path);
    let tx = fs_tx.clone();

    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let event = match res {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!(error = %e, "Filesystem watcher error");
                return;
            }
        };

        let kind = match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => FileChangeKind::Modified,
            EventKind::Remove(_) => FileChangeKind::Deleted,
            _ => return,
        };

        for path in &event.paths {
            // Get relative path
            let rel_path = match path.strip_prefix(&repo_path_buf) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let rel_str = match rel_path.to_str() {
                Some(s) => s,
                None => continue,
            };

            // Filter by ignored directories
            if rel_path.components().any(|c| {
                if let std::path::Component::Normal(name) = c {
                    if let Some(name_str) = name.to_str() {
                        return indexer::is_ignored_component(name_str);
                    }
                }
                false
            }) {
                continue;
            }

            // Filter by ignored suffixes
            if let Some(name) = rel_path.file_name().and_then(|n| n.to_str()) {
                if indexer::is_ignored_suffix(name) {
                    continue;
                }
            }

            // Filter by .gitignore
            if let Some(ref gi) = callback_gitignore {
                let is_dir = path.is_dir();
                if gi.matched(rel_path, is_dir).is_ignore() {
                    continue;
                }
            }

            let _ = tx.try_send(FsEvent {
                repo_path: repo_path_owned.clone(),
                rel_path: rel_str.to_string(),
                kind,
            });
        }
    });

    match watcher {
        Ok(mut w) => {
            if let Err(e) = w.watch(Path::new(repo_path), RecursiveMode::Recursive) {
                tracing::warn!(path = %repo_path, error = %e, "Failed to watch repo");
                return None;
            }
            tracing::info!(path = %repo_path, "Watching for file changes");
            Some(RepoWatcher {
                _watcher: w,
                ignore_patterns,
                max_file_size,
                gitignore,
            })
        }
        Err(e) => {
            tracing::warn!(path = %repo_path, error = %e, "Failed to create watcher");
            None
        }
    }
}

async fn event_loop(
    indexer: Arc<Indexer>,
    config_cache: Arc<ConfigCache>,
    mut cmd_rx: mpsc::Receiver<WatcherCommand>,
    fs_tx: mpsc::Sender<FsEvent>,
    mut fs_rx: mpsc::Receiver<FsEvent>,
    mut watchers: HashMap<String, RepoWatcher>,
) {
    let mut debouncer = Debouncer::new();

    loop {
        let sleep_duration = debouncer
            .time_until_flush()
            .unwrap_or(tokio::time::Duration::from_secs(3600));

        tokio::select! {
            // Handle commands from MCP server
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(WatcherCommand::Watch { repo_path }) => {
                        if !watchers.contains_key(&repo_path) {
                            if let Some(rw) = create_repo_watcher(&repo_path, &config_cache, &fs_tx) {
                                watchers.insert(repo_path.clone(), rw);
                            }
                        }
                    }
                    Some(WatcherCommand::Unwatch { repo_path }) => {
                        watchers.remove(&repo_path);
                        tracing::info!(path = %repo_path, "Stopped watching repo");
                    }
                    Some(WatcherCommand::Shutdown) | None => {
                        tracing::info!("Watcher shutting down");
                        break;
                    }
                }
            }

            // Handle filesystem events
            event = fs_rx.recv() => {
                if let Some(fs_event) = event {
                    // Check repo-specific ignore patterns
                    if let Some(rw) = watchers.get(&fs_event.repo_path) {
                        if indexer::matches_ignore_pattern(&fs_event.rel_path, &rw.ignore_patterns) {
                            continue;
                        }
                        if let Some(ref gi) = rw.gitignore {
                            let rel = Path::new(&fs_event.rel_path);
                            if gi.matched(rel, false).is_ignore() {
                                continue;
                            }
                        }
                    }

                    tracing::debug!(
                        repo = %fs_event.repo_path,
                        file = %fs_event.rel_path,
                        kind = ?fs_event.kind,
                        "File change detected"
                    );
                    debouncer.record(&fs_event.repo_path, &fs_event.rel_path, fs_event.kind);
                }
            }

            // Debounce timer
            _ = tokio::time::sleep(sleep_duration) => {
                if debouncer.is_ready() {
                    let batches = debouncer.flush();
                    for (repo_path, changes) in batches {
                        let rw = match watchers.get(&repo_path) {
                            Some(rw) => rw,
                            None => continue,
                        };

                        let modified: Vec<String> = changes
                            .iter()
                            .filter(|(_, kind)| **kind == FileChangeKind::Modified)
                            .map(|(path, _)| path.clone())
                            .collect();
                        let deleted: Vec<String> = changes
                            .iter()
                            .filter(|(_, kind)| **kind == FileChangeKind::Deleted)
                            .map(|(path, _)| path.clone())
                            .collect();

                        let total = modified.len() + deleted.len();
                        tracing::info!(
                            repo = %repo_path,
                            modified = modified.len(),
                            deleted = deleted.len(),
                            "Flushing {} file changes",
                            total
                        );

                        let indexer = indexer.clone();
                        let repo = repo_path.clone();
                        let ignore = rw.ignore_patterns.clone();
                        let max_size = rw.max_file_size;

                        tokio::task::spawn_blocking(move || {
                            let storage = match SqliteStorage::open(&repo) {
                                Ok(s) => s,
                                Err(e) => {
                                    tracing::warn!(repo = %repo, error = %e, "Failed to open storage for reindex");
                                    return;
                                }
                            };

                            // Handle modified files
                            if !modified.is_empty() {
                                match indexer.update_files(&repo, &modified, &storage, &ignore, max_size) {
                                    Ok(stats) => {
                                        tracing::info!(
                                            repo = %repo,
                                            files = stats.files_changed,
                                            symbols = stats.symbols_count,
                                            duration_ms = stats.duration_ms,
                                            "Background reindex complete"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(repo = %repo, error = %e, "Background reindex failed");
                                    }
                                }
                            }

                            // Handle deleted files
                            if !deleted.is_empty() {
                                match delete_files(&repo, &deleted, &storage) {
                                    Ok(count) => {
                                        tracing::info!(repo = %repo, deleted = count, "Removed deleted files from index");
                                    }
                                    Err(e) => {
                                        tracing::warn!(repo = %repo, error = %e, "Failed to remove deleted files");
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}

fn delete_files(repo_path: &str, rel_paths: &[String], storage: &SqliteStorage) -> Result<usize> {
    storage.delete_files_by_rel_paths(repo_path, rel_paths)
}
