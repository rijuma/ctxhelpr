pub mod indexing_tracker;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::serde;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

use crate::config::{ConfigCache, OutputConfig};
use crate::indexer::Indexer;
use crate::output::{CompactFormatter, OutputFormatter, TokenBudget};
use crate::storage::{self, SqliteStorage};
use crate::watcher::WatcherHandle;

use self::indexing_tracker::IndexingTracker;

type McpError = rmcp::ErrorData;

// ── Parameter structs ────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RepoPathParams {
    /// Absolute path to the repository root
    pub path: String,
    /// Optional token budget — limits response size (approximate, 1 token ≈ 4 bytes)
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileSymbolsParams {
    /// Absolute path to the repository root
    pub path: String,
    /// Relative file path within the repo
    pub file: String,
    /// Optional token budget — limits response size (approximate, 1 token ≈ 4 bytes)
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolIdParams {
    /// Absolute path to the repository root
    pub path: String,
    /// Symbol ID from a previous query
    pub symbol_id: i64,
    /// Optional token budget — limits response size (approximate, 1 token ≈ 4 bytes)
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReposParams {
    /// Optional token budget — limits response size (approximate, 1 token ≈ 4 bytes)
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteReposParams {
    /// Absolute paths of repositories to delete indexes for. If empty, does nothing.
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Absolute path to the repository root
    pub path: String,
    /// Search query (supports FTS5 syntax: AND, OR, NOT, prefix*)
    pub query: String,
    /// Optional token budget — limits response size (approximate, 1 token ≈ 4 bytes)
    pub max_tokens: Option<usize>,
}

fn resolve_budget(param_budget: Option<usize>, config_budget: Option<usize>) -> Option<usize> {
    param_budget.or(config_budget)
}

fn apply_budget(output: String, max_tokens: Option<usize>, array_key: &str) -> String {
    match max_tokens {
        Some(limit) => TokenBudget::from_tokens(limit).truncate_json(&output, array_key),
        None => output,
    }
}

// ── Server ────────────────────────────────────────────────

fn formatter(config: &OutputConfig) -> CompactFormatter {
    CompactFormatter::new(config)
}

#[derive(Clone)]
pub struct CtxhelprServer {
    indexer: Arc<Indexer>,
    config_cache: Arc<ConfigCache>,
    watcher: WatcherHandle,
    indexing_tracker: Arc<IndexingTracker>,
    tool_router: ToolRouter<Self>,
}

fn open_storage(path: &str) -> Result<SqliteStorage, McpError> {
    SqliteStorage::open(path)
        .map_err(|e| McpError::internal_error(format!("Storage error for {path}: {e}"), None))
}

#[tool_router]
impl CtxhelprServer {
    pub fn new(
        indexer: Arc<Indexer>,
        config_cache: Arc<ConfigCache>,
        watcher: WatcherHandle,
    ) -> Self {
        Self {
            indexer,
            config_cache,
            watcher,
            indexing_tracker: Arc::new(IndexingTracker::new()),
            tool_router: Self::tool_router(),
        }
    }

    fn auto_index_message(path: &str, status: &str) -> CallToolResult {
        let msg = format!(
            "Repository '{path}' is not indexed yet ({status}). \
             Background indexing has been triggered.\n\n\
             Options:\n\
             1. Call `index_repository` with this path to wait for indexing to complete, then retry.\n\
             2. Use Grep/Glob/Read as fallback tools for now and retry ctxhelpr tools later."
        );
        CallToolResult::error(vec![Content::text(msg)])
    }

    fn ensure_indexed(&self, path: &str) -> Option<CallToolResult> {
        if !storage::has_index_db(path) {
            return Some(self.trigger_background_index(path));
        }
        let storage = match SqliteStorage::open(path) {
            Ok(s) => s,
            Err(_) => return Some(self.trigger_background_index(path)),
        };
        if storage.is_repo_indexed(path) {
            return None;
        }
        Some(self.trigger_background_index(path))
    }

    fn trigger_background_index(&self, path: &str) -> CallToolResult {
        if self.indexing_tracker.is_indexing(path) {
            return Self::auto_index_message(path, "currently being indexed");
        }

        let handle = match self.indexing_tracker.start_indexing(path) {
            Some(h) => h,
            None => return Self::auto_index_message(path, "currently being indexed"),
        };

        let indexer = self.indexer.clone();
        let config_cache = self.config_cache.clone();
        let watcher = self.watcher.clone();
        let path_owned = path.to_string();

        tokio::spawn(async move {
            let idx = indexer.clone();
            let p = path_owned.clone();
            let config = config_cache.get(&p);
            let ignore = config.indexer.ignore.clone();
            let max_file_size = config.indexer.max_file_size;

            let result = tokio::task::spawn_blocking(move || {
                let storage = SqliteStorage::open(&p)?;
                idx.index(&p, &storage, &ignore, max_file_size)
            })
            .await;

            match result {
                Ok(Ok(stats)) => {
                    tracing::info!(
                        path = %path_owned,
                        files = stats.files_total,
                        symbols = stats.symbols_count,
                        duration_ms = stats.duration_ms,
                        "Background auto-index complete"
                    );
                    crate::skills::refresh(&crate::skills::base_dirs_for_repo(&path_owned));
                    watcher.watch_repo(&path_owned).await;
                }
                Ok(Err(e)) => {
                    tracing::warn!(path = %path_owned, error = %e, "Background auto-index failed");
                }
                Err(e) => {
                    tracing::warn!(path = %path_owned, error = %e, "Background auto-index task panicked");
                }
            }

            handle.complete();
        });

        Self::auto_index_message(path, "indexing started")
    }

    #[tool(
        description = "Index or re-index a repository. Incrementally updates: only re-parses files whose content hash has changed. Detects new and deleted files."
    )]
    async fn index_repository(
        &self,
        Parameters(params): Parameters<RepoPathParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, "index_repository");

        // Wait for background auto-indexing to finish (if any) to avoid concurrent writes
        if let Some(mut rx) = self.indexing_tracker.wait_for_completion(&params.path) {
            tracing::info!(path = %params.path, "Waiting for background auto-index to complete");
            while !*rx.borrow() {
                if rx.changed().await.is_err() {
                    break;
                }
            }
        }

        let config = self.config_cache.get(&params.path);
        let indexer = self.indexer.clone();
        let path = params.path.clone();
        let ignore = config.indexer.ignore.clone();
        let max_file_size = config.indexer.max_file_size;
        let stats = tokio::task::spawn_blocking(move || {
            let storage = open_storage(&path)?;
            indexer
                .index(&path, &storage, &ignore, max_file_size)
                .map_err(|e| McpError::internal_error(format!("Indexing failed: {e}"), None))
        })
        .await
        .map_err(|e| McpError::internal_error(format!("Indexing task failed: {e}"), None))??;
        crate::skills::refresh(&crate::skills::base_dirs_for_repo(&params.path));
        self.watcher.watch_repo(&params.path).await;
        let fmt = formatter(&config.output);
        Ok(CallToolResult::success(vec![Content::text(
            fmt.format_index_result(&stats),
        )]))
    }

    #[tool(
        description = "PREFER as the first step when exploring any indexed repository. Returns languages, module structure, key types, and entry points in one call -- replaces multiple Glob/Read calls to understand project layout. Output key legend: n=name k=kind f=file l=lines id=symbol_id sig=signature doc=doc_comment p=path"
    )]
    async fn get_overview(
        &self,
        Parameters(params): Parameters<RepoPathParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, "get_overview");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let data = storage
            .get_overview(&params.path)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(fmt.format_overview(&data), budget, "top_types");
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "PREFER over Read for understanding a file's structure. Lists all functions, types, imports with signatures and line ranges -- more concise than reading raw source. Returns symbol IDs for drill-down into details, references, and dependencies."
    )]
    async fn get_file_symbols(
        &self,
        Parameters(params): Parameters<FileSymbolsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, file = %params.file, "get_file_symbols");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let symbols = storage
            .get_file_symbols(&params.path, &params.file)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            fmt.format_file_symbols(&params.file, &symbols),
            budget,
            "syms",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "PREFER over Read for inspecting a specific function, class, or type. Returns signature, doc comment, call graph (what it calls and who calls it), and type references in a single call."
    )]
    async fn get_symbol_detail(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_symbol_detail");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let sym = storage
            .get_symbol_detail(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let all_deps = storage
            .get_dependencies(&params.path, params.symbol_id)
            .unwrap_or_else(|err| {
                tracing::warn!(symbol_id = params.symbol_id, error = %err, "Failed to get dependencies");
                Vec::new()
            });
        let (type_refs, calls): (Vec<_>, Vec<_>) =
            all_deps.into_iter().partition(|r| r.ref_kind == "type_ref");
        let called_by = storage
            .get_references(&params.path, params.symbol_id)
            .unwrap_or_else(|err| {
                tracing::warn!(symbol_id = params.symbol_id, error = %err, "Failed to get references");
                Vec::new()
            });
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            fmt.format_symbol_detail(&sym, &calls, &called_by, &type_refs),
            budget,
            "called_by",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "PREFER over Grep for finding functions, classes, types, and symbols by name. Returns structured results with signatures, file locations, and symbol IDs for drill-down. Supports prefix* matching, AND/OR/NOT operators. Use Grep only for non-symbol text patterns."
    )]
    async fn search_symbols(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, query = %params.query, "search_symbols");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let results = storage
            .search_symbols(&params.path, &params.query, config.search.max_results)
            .map_err(|e| McpError::internal_error(format!("Search failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            fmt.format_search_results(&params.query, &results),
            budget,
            "hits",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "PREFER over Grep for finding callers and usages of a symbol. Returns semantically accurate references (callers, importers, type references) -- unlike text search, never returns false positives from comments or strings."
    )]
    async fn get_references(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_references");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let refs = storage
            .get_references(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            fmt.format_references(params.symbol_id, &refs),
            budget,
            "refs_to",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Find all symbols a given symbol depends on: called functions, imported modules, referenced types. Not possible with text search -- requires semantic analysis from the index."
    )]
    async fn get_dependencies(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_dependencies");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let deps = storage
            .get_dependencies(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            fmt.format_dependencies(params.symbol_id, &deps),
            budget,
            "deps",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Check index freshness and statistics: when last indexed, file/symbol/reference counts, stale and deleted files."
    )]
    async fn index_status(
        &self,
        Parameters(params): Parameters<RepoPathParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, "index_status");
        if let Some(result) = self.ensure_indexed(&params.path) {
            return Ok(result);
        }
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let fmt = formatter(&config.output);
        let status = storage
            .get_index_status(&params.path)
            .map_err(|e| McpError::internal_error(format!("Status failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            fmt.format_index_status(&status),
        )]))
    }

    #[tool(
        description = "List all indexed repositories with stats: path, last indexed time, file/symbol counts, database size."
    )]
    async fn list_repos(
        &self,
        Parameters(params): Parameters<ListReposParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("list_repos");
        let repos = storage::list_indexed_repos()
            .map_err(|e| McpError::internal_error(format!("Failed to list repos: {e}"), None))?;

        let items: Vec<serde_json::Value> = repos
            .iter()
            .map(|r| {
                let mut v = serde_json::json!({
                    "path": r.abs_path,
                    "files": r.file_count,
                    "symbols": r.symbol_count,
                    "db_size": r.db_size_bytes,
                });
                if let Some(at) = &r.last_indexed_at {
                    v["last_indexed"] = serde_json::json!(at);
                }
                v
            })
            .collect();

        let output = serde_json::json!({"repos": items, "total": repos.len()}).to_string();
        let output = apply_budget(output, params.max_tokens, "repos");
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Delete indexed repository data. Removes the database files for the specified repositories."
    )]
    async fn delete_repos(
        &self,
        Parameters(params): Parameters<DeleteReposParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(count = params.paths.len(), "delete_repos");
        if params.paths.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                r#"{"deleted":0}"#.to_string(),
            )]));
        }

        let mut deleted = 0;
        let mut errors: Vec<String> = Vec::new();
        for path in &params.paths {
            match storage::delete_repo_index(path) {
                Ok(()) => {
                    self.watcher.unwatch_repo(path).await;
                    deleted += 1;
                }
                Err(e) => errors.push(format!("{path}: {e}")),
            }
        }

        let mut result = serde_json::json!({"deleted": deleted});
        if !errors.is_empty() {
            result["errors"] = serde_json::json!(errors);
        }
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for CtxhelprServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "PREFER ctxhelpr tools over Grep/Glob/Read for code navigation tasks \
                 (finding functions, classes, types, tracing calls, understanding structure). \
                 ctxhelpr returns structured symbol data with signatures, call graphs, and \
                 cross-references in a single call -- faster and more accurate than text search. \
                 Workflow: get_overview -> drill with search_symbols/get_file_symbols/\
                 get_symbol_detail/get_references/get_dependencies. \
                 The index is kept fresh automatically via background file watching -- no manual \
                 update calls needed. The index only includes git-tracked files (.gitignore is respected). \
                 For gitignored files, use Grep/Glob/Read. \
                 Use Grep/Glob only for non-code searches (config files, text patterns, log messages). \
                 Output keys: n=name k=kind f=file l=lines id=symbol_id sig=signature doc=doc_comment p=path"
                    .into(),
            ),
        }
    }
}
