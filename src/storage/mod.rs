pub mod tokenizer;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

use crate::indexer::{ExtractedRef, ExtractedSymbol};

use self::tokenizer::split_code_identifier;

const SCHEMA: &str = include_str!("schema.sql");

/// Data structures returned by queries

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields accessed by indexer for incremental updates
pub struct FileRecord {
    pub id: i64,
    pub rel_path: String,
    pub content_hash: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct SymbolRecord {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub start_line: i64,
    pub end_line: i64,
    pub file_rel_path: String,
    pub parent_symbol_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RefRecord {
    pub from_symbol_id: i64,
    pub from_name: Option<String>,
    pub from_file: Option<String>,
    pub to_symbol_id: Option<i64>,
    pub to_name: String,
    pub ref_kind: String,
    pub line: Option<i64>,
}

impl RefRecord {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            from_symbol_id: row.get(0)?,
            from_name: row.get(1)?,
            from_file: row.get(2)?,
            to_symbol_id: row.get(3)?,
            to_name: row.get(4)?,
            ref_kind: row.get(5)?,
            line: row.get(6)?,
        })
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields available for future output format enhancements
pub struct SearchHit {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub file_rel_path: String,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub start_line: i64,
    pub end_line: i64,
    pub rank: f64,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: String,
    pub file_count: i64,
    pub symbol_count: i64,
}

#[derive(Debug, Clone)]
pub struct OverviewData {
    pub repo_name: String,
    pub languages: Vec<(String, i64)>,
    pub modules: Vec<ModuleInfo>,
    pub top_types: Vec<SymbolRecord>,
    pub entry_points: Vec<SymbolRecord>,
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub files_total: usize,
    pub files_new: usize,
    pub files_changed: usize,
    pub files_unchanged: usize,
    pub files_deleted: usize,
    pub symbols_count: usize,
    pub refs_count: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub repo_path: String,
    pub indexed_at: Option<String>,
    pub total_files: i64,
    pub total_symbols: i64,
    pub total_refs: i64,
    pub stale_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub languages: Vec<(String, i64)>,
}

impl SymbolRecord {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            signature: row.get(3)?,
            doc_comment: row.get(4)?,
            start_line: row.get(5)?,
            end_line: row.get(6)?,
            file_rel_path: row.get(7)?,
            parent_symbol_id: row.get(8)?,
        })
    }
}

fn rows_empty(conn: &Connection, table: &str) -> Result<bool> {
    let count: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })?;
    Ok(count == 0)
}

pub fn db_path_for_repo(repo_path: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    let hash = hex::encode(Sha256::digest(repo_path.as_bytes()));
    let short_hash = &hash[..16];
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    cache_dir
        .join("ctxhelpr")
        .join(format!("{}.db", short_hash))
}

fn cache_dir() -> PathBuf {
    let dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    dir.join("ctxhelpr")
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields available for future CLI and MCP tool output
pub struct RepoInfo {
    pub abs_path: String,
    pub last_indexed_at: Option<String>,
    pub created_at: String,
    pub file_count: i64,
    pub symbol_count: i64,
    pub db_path: PathBuf,
    pub db_size_bytes: u64,
}

/// List all indexed repos by scanning cache directory DB files.
pub fn list_indexed_repos() -> Result<Vec<RepoInfo>> {
    let dir = cache_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }

        let conn = match Connection::open(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check if this is a ctxhelpr database with the repositories table
        let has_repos: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repositories'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_repos {
            continue;
        }

        let mut stmt =
            match conn.prepare("SELECT abs_path, last_indexed_at, created_at FROM repositories") {
                Ok(s) => s,
                Err(_) => continue,
            };

        let rows: Vec<(String, Option<String>, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .ok()
            .map(|r| r.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        for (abs_path, last_indexed_at, created_at) in rows {
            let file_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
                .unwrap_or(0);
            let symbol_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))
                .unwrap_or(0);
            let db_size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            repos.push(RepoInfo {
                abs_path,
                last_indexed_at,
                created_at,
                file_count,
                symbol_count,
                db_path: path.clone(),
                db_size_bytes,
            });
        }
    }

    repos.sort_by(|a, b| a.abs_path.cmp(&b.abs_path));
    Ok(repos)
}

/// Delete a repo's index (remove DB file and WAL/SHM files).
pub fn delete_repo_index(repo_path: &str) -> Result<()> {
    let db_path = db_path_for_repo(repo_path);
    if !db_path.exists() {
        anyhow::bail!("No index found for {repo_path}");
    }
    remove_db_files(&db_path)
}

/// Delete all indexed repos (remove all DB files in cache dir).
pub fn delete_all_repo_indexes() -> Result<usize> {
    let dir = cache_dir();
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("db") {
            remove_db_files(&path)?;
            count += 1;
        }
    }
    Ok(count)
}

fn remove_db_files(db_path: &Path) -> Result<()> {
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
    }
    let shm = db_path.with_extension("db-shm");
    if shm.exists() {
        std::fs::remove_file(shm)?;
    }
    let wal = db_path.with_extension("db-wal");
    if wal.exists() {
        std::fs::remove_file(wal)?;
    }
    Ok(())
}

pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    pub fn open(repo_path: &str) -> Result<Self> {
        let db_path = db_path_for_repo(repo_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    #[allow(dead_code)] // Used by integration tests
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&self) -> Result<()> {
        // Check if this is an existing DB that needs migration
        let has_name_tokens = self.has_column("symbols", "name_tokens");
        let has_metadata = self.has_table("metadata");

        if !has_metadata || !has_name_tokens {
            // Either fresh DB or pre-migration DB — apply full schema
            // For existing DBs, we need to add new columns/tables before re-creating triggers
            if !has_name_tokens && self.has_table("symbols") {
                // Old DB: add name_tokens column and rebuild FTS
                self.conn
                    .execute_batch("ALTER TABLE symbols ADD COLUMN name_tokens TEXT")
                    .context("Failed to add name_tokens column")?;

                // Populate name_tokens for existing symbols
                let mut stmt = self.conn.prepare("SELECT id, name FROM symbols")?;
                let rows: Vec<(i64, String)> = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                drop(stmt);

                for (id, name) in &rows {
                    let tokens = split_code_identifier(name);
                    self.conn.execute(
                        "UPDATE symbols SET name_tokens = ?1 WHERE id = ?2",
                        params![tokens, id],
                    )?;
                }

                // Drop old FTS table and triggers, let schema re-create them
                self.conn.execute_batch(
                    "DROP TRIGGER IF EXISTS symbols_ai;
                     DROP TRIGGER IF EXISTS symbols_ad;
                     DROP TRIGGER IF EXISTS symbols_au;
                     DROP TABLE IF EXISTS fts_symbols;",
                )?;
            }

            // Apply full schema (CREATE IF NOT EXISTS is safe for existing tables)
            self.conn
                .execute_batch(SCHEMA)
                .context("Failed to initialize database schema")?;

            // Rebuild FTS content if we migrated an existing DB
            if !has_name_tokens && !rows_empty(&self.conn, "symbols")? {
                self.conn
                    .execute_batch("INSERT INTO fts_symbols(fts_symbols) VALUES('rebuild')")?;
            }

            // Set schema version
            self.conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '2')",
                [],
            )?;
        }

        Ok(())
    }

    fn has_column(&self, table: &str, column: &str) -> bool {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info(?) WHERE name = ?",
                params![table, column],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false)
    }

    fn has_table(&self, table: &str) -> bool {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?",
                params![table],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false)
    }

    // ── Transaction control ──

    pub fn begin_transaction(&self) -> Result<()> {
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .context("Failed to begin transaction")?;
        Ok(())
    }

    pub fn commit(&self) -> Result<()> {
        self.conn
            .execute_batch("COMMIT")
            .context("Failed to commit transaction")?;
        Ok(())
    }

    // ── Repository operations ──

    pub fn ensure_repo(&self, abs_path: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT OR IGNORE INTO repositories (abs_path) VALUES (?1)",
            params![abs_path],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM repositories WHERE abs_path = ?1",
            params![abs_path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn update_repo_timestamp(&self, repo_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE repositories SET last_indexed_at = datetime('now') WHERE id = ?1",
            params![repo_id],
        )?;
        Ok(())
    }

    // ── File operations ──

    pub fn get_files(&self, repo_id: i64) -> Result<Vec<FileRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, rel_path, content_hash, language FROM files WHERE repo_id = ?1")?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                rel_path: row.get(1)?,
                content_hash: row.get(2)?,
                language: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn upsert_file(
        &self,
        repo_id: i64,
        rel_path: &str,
        content_hash: &str,
        language: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO files (repo_id, rel_path, content_hash, language)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(repo_id, rel_path)
             DO UPDATE SET content_hash = ?3, language = ?4, last_indexed_at = datetime('now')",
            params![repo_id, rel_path, content_hash, language],
        )?;
        let id = self.conn.query_row(
            "SELECT id FROM files WHERE repo_id = ?1 AND rel_path = ?2",
            params![repo_id, rel_path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn delete_file(&self, file_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
        Ok(())
    }

    // ── Symbol operations ──

    pub fn clear_file_symbols(&self, file_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
        Ok(())
    }

    pub fn insert_symbol(
        &self,
        file_id: i64,
        repo_id: i64,
        file_rel_path: &str,
        sym: &ExtractedSymbol,
        parent_id: Option<i64>,
    ) -> Result<i64> {
        let name_tokens = split_code_identifier(&sym.name);
        self.conn.execute(
            "INSERT INTO symbols (file_id, name, kind, signature, doc_comment, start_line, end_line, parent_symbol_id, file_rel_path, repo_id, name_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                file_id,
                sym.name,
                sym.kind.as_str(),
                sym.signature,
                sym.doc_comment,
                sym.start_line as i64,
                sym.end_line as i64,
                parent_id,
                file_rel_path,
                repo_id,
                name_tokens,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_ref(&self, from_symbol_id: i64, r: &ExtractedRef) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO refs (from_symbol_id, to_name, ref_kind, line)
             VALUES (?1, ?2, ?3, ?4)",
            params![from_symbol_id, r.name, r.kind.as_str(), r.line as i64],
        )?;
        Ok(())
    }

    /// Insert a symbol and all its children/references recursively
    pub fn insert_symbol_tree(
        &self,
        file_id: i64,
        repo_id: i64,
        file_rel_path: &str,
        sym: &ExtractedSymbol,
        parent_id: Option<i64>,
    ) -> Result<()> {
        let sym_id = self.insert_symbol(file_id, repo_id, file_rel_path, sym, parent_id)?;
        for r in &sym.references {
            self.insert_ref(sym_id, r)?;
        }
        for child in &sym.children {
            self.insert_symbol_tree(file_id, repo_id, file_rel_path, child, Some(sym_id))?;
        }
        Ok(())
    }

    // ── Reference resolution ──

    pub fn resolve_references(&self, repo_id: i64) -> Result<usize> {
        let updated = self.conn.execute(
            "UPDATE refs SET to_symbol_id = (
                SELECT s.id FROM symbols s
                WHERE s.name = refs.to_name AND s.repo_id = ?1
                LIMIT 1
             )
             WHERE to_symbol_id IS NULL
             AND from_symbol_id IN (SELECT id FROM symbols WHERE repo_id = ?1)",
            params![repo_id],
        )?;
        Ok(updated)
    }

    // ── Query operations ──

    pub fn get_overview(&self, repo_path: &str) -> Result<OverviewData> {
        let repo_id: i64 = self
            .conn
            .query_row(
                "SELECT id FROM repositories WHERE abs_path = ?1",
                params![repo_path],
                |row| row.get(0),
            )
            .context("Repository not indexed. Run index_repository first.")?;

        let repo_name = repo_path
            .rsplit('/')
            .next()
            .unwrap_or(repo_path)
            .to_string();

        // Languages
        let mut stmt = self.conn.prepare(
            "SELECT language, COUNT(*) FROM files WHERE repo_id = ?1 GROUP BY language ORDER BY COUNT(*) DESC",
        )?;
        let languages: Vec<(String, i64)> = stmt
            .query_map(params![repo_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        // Modules (directory-level aggregation)
        let mut stmt = self.conn.prepare(
            "SELECT
                CASE
                    WHEN INSTR(rel_path, '/') > 0 THEN SUBSTR(rel_path, 1, INSTR(rel_path, '/'))
                    ELSE './'
                END as dir,
                COUNT(DISTINCT f.id) as file_count,
                COUNT(s.id) as sym_count
             FROM files f
             LEFT JOIN symbols s ON s.file_id = f.id
             WHERE f.repo_id = ?1
             GROUP BY dir
             ORDER BY sym_count DESC
             LIMIT 20",
        )?;
        let modules: Vec<ModuleInfo> = stmt
            .query_map(params![repo_id], |row| {
                Ok(ModuleInfo {
                    path: row.get(0)?,
                    file_count: row.get(1)?,
                    symbol_count: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Top types (classes, interfaces, structs, enums, traits)
        let top_types = self.query_symbols_where(
            repo_id,
            "kind",
            &["class", "interface", "struct", "enum", "trait"],
            "ORDER BY (end_line - start_line) DESC",
            10,
        )?;

        // Entry points (main functions, exported functions at top level)
        let entry_points =
            self.query_symbols_where(repo_id, "name", &["main", "index", "app", "server"], "", 5)?;

        Ok(OverviewData {
            repo_name,
            languages,
            modules,
            top_types,
            entry_points,
        })
    }

    fn query_symbols_where(
        &self,
        repo_id: i64,
        column: &str,
        values: &[&str],
        order_by: &str,
        limit: usize,
    ) -> Result<Vec<SymbolRecord>> {
        const VALID_COLUMNS: &[&str] =
            &["name", "kind", "file_rel_path", "signature", "doc_comment"];
        anyhow::ensure!(
            VALID_COLUMNS.contains(&column),
            "Invalid column for symbol query: {column}"
        );

        const VALID_ORDER_BY: &[&str] = &[
            "",
            "ORDER BY (end_line - start_line) DESC",
            "ORDER BY start_line",
            "ORDER BY name",
        ];
        anyhow::ensure!(
            VALID_ORDER_BY.contains(&order_by),
            "Invalid order_by clause: {order_by}"
        );

        let placeholders: Vec<String> = (0..values.len()).map(|i| format!("?{}", i + 2)).collect();
        let sql = format!(
            "SELECT id, name, kind, signature, doc_comment, start_line, end_line, file_rel_path, parent_symbol_id
             FROM symbols WHERE repo_id = ?1 AND {column} IN ({})
             {order_by} LIMIT {limit}",
            placeholders.join(","),
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(repo_id)];
        for v in values {
            param_values.push(Box::new(v.to_string()));
        }
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), SymbolRecord::from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_file_symbols(&self, repo_path: &str, file: &str) -> Result<Vec<SymbolRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.kind, s.signature, s.doc_comment, s.start_line, s.end_line, s.file_rel_path, s.parent_symbol_id
             FROM symbols s
             JOIN repositories r ON s.repo_id = r.id
             WHERE r.abs_path = ?1 AND s.file_rel_path = ?2
             ORDER BY s.start_line",
        )?;
        let rows = stmt.query_map(params![repo_path, file], SymbolRecord::from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_symbol_detail(&self, repo_path: &str, symbol_id: i64) -> Result<SymbolRecord> {
        self.conn.query_row(
            "SELECT s.id, s.name, s.kind, s.signature, s.doc_comment, s.start_line, s.end_line, s.file_rel_path, s.parent_symbol_id
             FROM symbols s
             JOIN repositories r ON s.repo_id = r.id
             WHERE r.abs_path = ?1 AND s.id = ?2",
            params![repo_path, symbol_id],
            SymbolRecord::from_row,
        ).context("Symbol not found")
    }

    pub fn search_symbols(&self, repo_path: &str, query: &str) -> Result<Vec<SearchHit>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.kind, s.file_rel_path, s.signature, s.doc_comment, s.start_line, s.end_line, rank
             FROM fts_symbols fts
             JOIN symbols s ON s.id = fts.rowid
             JOIN repositories r ON s.repo_id = r.id
             WHERE r.abs_path = ?1 AND fts_symbols MATCH ?2
             ORDER BY rank
             LIMIT 20",
        )?;
        let rows = stmt.query_map(params![repo_path, query], |row| {
            Ok(SearchHit {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file_rel_path: row.get(3)?,
                signature: row.get(4)?,
                doc_comment: row.get(5)?,
                start_line: row.get(6)?,
                end_line: row.get(7)?,
                rank: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_references(&self, repo_path: &str, symbol_id: i64) -> Result<Vec<RefRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.from_symbol_id, s.name, s.file_rel_path, r.to_symbol_id, r.to_name, r.ref_kind, r.line
             FROM refs r
             JOIN symbols s ON s.id = r.from_symbol_id
             JOIN repositories repo ON s.repo_id = repo.id
             WHERE repo.abs_path = ?1 AND r.to_symbol_id = ?2
             ORDER BY s.file_rel_path, r.line",
        )?;
        let rows = stmt.query_map(params![repo_path, symbol_id], RefRecord::from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_dependencies(&self, repo_path: &str, symbol_id: i64) -> Result<Vec<RefRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.from_symbol_id, NULL, NULL, r.to_symbol_id, r.to_name, r.ref_kind, r.line
             FROM refs r
             JOIN symbols s ON s.id = r.from_symbol_id
             JOIN repositories repo ON s.repo_id = repo.id
             WHERE repo.abs_path = ?1 AND r.from_symbol_id = ?2
             ORDER BY r.ref_kind, r.to_name",
        )?;
        let rows = stmt.query_map(params![repo_path, symbol_id], RefRecord::from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_index_status(&self, repo_path: &str) -> Result<IndexStatus> {
        let (repo_id, indexed_at): (i64, Option<String>) = self
            .conn
            .query_row(
                "SELECT id, last_indexed_at FROM repositories WHERE abs_path = ?1",
                params![repo_path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .context("Repository not indexed")?;

        let total_files: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM files WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0),
        )?;

        let total_symbols: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE repo_id = ?1",
            params![repo_id],
            |row| row.get(0),
        )?;

        let total_refs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM refs WHERE from_symbol_id IN (SELECT id FROM symbols WHERE repo_id = ?1)",
            params![repo_id],
            |row| row.get(0),
        )?;

        let mut stmt = self
            .conn
            .prepare("SELECT language, COUNT(*) FROM files WHERE repo_id = ?1 GROUP BY language")?;
        let languages: Vec<(String, i64)> = stmt
            .query_map(params![repo_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(IndexStatus {
            repo_path: repo_path.to_string(),
            indexed_at,
            total_files,
            total_symbols,
            total_refs,
            stale_files: Vec::new(), // Populated by indexer during status check
            deleted_files: Vec::new(),
            languages,
        })
    }
}
