use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::serde;
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

use crate::config::ConfigCache;
use crate::indexer::Indexer;
use crate::output::{CompactFormatter, OutputFormatter, TokenBudget};
use crate::storage::SqliteStorage;

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
pub struct UpdateFilesParams {
    /// Absolute path to the repository root
    pub path: String,
    /// List of relative file paths to re-index
    pub files: Vec<String>,
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

#[derive(Clone)]
pub struct CtxhelprServer {
    indexer: Arc<Indexer>,
    formatter: Arc<dyn OutputFormatter>,
    config_cache: Arc<ConfigCache>,
    tool_router: ToolRouter<Self>,
}

fn open_storage(path: &str) -> Result<SqliteStorage, McpError> {
    SqliteStorage::open(path)
        .map_err(|e| McpError::internal_error(format!("Storage error for {path}: {e}"), None))
}

#[tool_router]
impl CtxhelprServer {
    pub fn new() -> Self {
        Self {
            indexer: Arc::new(Indexer::new()),
            formatter: Arc::new(CompactFormatter),
            config_cache: Arc::new(ConfigCache::new()),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Index or re-index a repository. Incrementally updates: only re-parses files whose content hash has changed. Detects new and deleted files."
    )]
    async fn index_repository(
        &self,
        Parameters(params): Parameters<RepoPathParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, "index_repository");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let stats = self
            .indexer
            .index(&params.path, &storage, &config.indexer.ignore)
            .map_err(|e| McpError::internal_error(format!("Indexing failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            self.formatter.format_index_result(&stats),
        )]))
    }

    #[tool(
        description = "Re-index specific files after editing. Fast (~50ms per file), no full repo walk. Call this after completing edits."
    )]
    async fn update_files(
        &self,
        Parameters(params): Parameters<UpdateFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, file_count = params.files.len(), "update_files");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let stats = self
            .indexer
            .update_files(
                &params.path,
                &params.files,
                &storage,
                &config.indexer.ignore,
            )
            .map_err(|e| McpError::internal_error(format!("Update failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            self.formatter.format_update_result(&stats),
        )]))
    }

    #[tool(
        description = "Get high-level overview of an indexed repo: languages, module structure, key types, entry points. Output key legend: n=name k=kind f=file l=lines id=symbol_id sig=signature doc=doc_comment p=path"
    )]
    async fn get_overview(
        &self,
        Parameters(params): Parameters<RepoPathParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, "get_overview");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let data = storage
            .get_overview(&params.path)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(self.formatter.format_overview(&data), budget, "top_types");
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "List all symbols in a file: functions, types, imports with signatures and line ranges. Use symbol IDs to drill into details."
    )]
    async fn get_file_symbols(
        &self,
        Parameters(params): Parameters<FileSymbolsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, file = %params.file, "get_file_symbols");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let symbols = storage
            .get_file_symbols(&params.path, &params.file)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            self.formatter.format_file_symbols(&params.file, &symbols),
            budget,
            "syms",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Get full detail of a symbol: signature, doc comment, what it calls, who calls it, type references."
    )]
    async fn get_symbol_detail(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_symbol_detail");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let sym = storage
            .get_symbol_detail(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let calls = storage
            .get_dependencies(&params.path, params.symbol_id)
            .unwrap_or_else(|err| {
                tracing::warn!(symbol_id = params.symbol_id, error = %err, "Failed to get dependencies");
                Vec::new()
            });
        let called_by = storage
            .get_references(&params.path, params.symbol_id)
            .unwrap_or_else(|err| {
                tracing::warn!(symbol_id = params.symbol_id, error = %err, "Failed to get references");
                Vec::new()
            });
        let type_refs = Vec::new();
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            self.formatter
                .format_symbol_detail(&sym, &calls, &called_by, &type_refs),
            budget,
            "called_by",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Full-text search across all symbol names and doc comments. Supports: prefix* matching, AND/OR/NOT operators. Returns ranked results."
    )]
    async fn search_symbols(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, query = %params.query, "search_symbols");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let mut results = storage
            .search_symbols(&params.path, &params.query)
            .map_err(|e| McpError::internal_error(format!("Search failed: {e}"), None))?;
        results.truncate(config.search.max_results);
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            self.formatter
                .format_search_results(&params.query, &results),
            budget,
            "hits",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Find all symbols that reference a given symbol: callers, importers, type references."
    )]
    async fn get_references(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_references");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let refs = storage
            .get_references(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            self.formatter.format_references(params.symbol_id, &refs),
            budget,
            "refs_to",
        );
        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        description = "Find all symbols that a given symbol depends on: called functions, imported modules, referenced types."
    )]
    async fn get_dependencies(
        &self,
        Parameters(params): Parameters<SymbolIdParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(path = %params.path, symbol_id = params.symbol_id, "get_dependencies");
        let config = self.config_cache.get(&params.path);
        let storage = open_storage(&params.path)?;
        let deps = storage
            .get_dependencies(&params.path, params.symbol_id)
            .map_err(|e| McpError::internal_error(format!("Query failed: {e}"), None))?;
        let budget = resolve_budget(params.max_tokens, config.output.max_tokens);
        let output = apply_budget(
            self.formatter.format_dependencies(params.symbol_id, &deps),
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
        let storage = open_storage(&params.path)?;
        let status = storage
            .get_index_status(&params.path)
            .map_err(|e| McpError::internal_error(format!("Status failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            self.formatter.format_index_status(&status),
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
                "Semantic code index for fast context rebuilding. \
                 Workflow: index_repository -> get_overview -> drill with \
                 get_file_symbols/get_symbol_detail/search_symbols. \
                 After edits, call update_files to keep index fresh. \
                 Output key legend: n=name k=kind f=file l=lines(start-end) \
                 id=symbol_id sig=signature doc=doc_comment p=path"
                    .into(),
            ),
        }
    }
}
