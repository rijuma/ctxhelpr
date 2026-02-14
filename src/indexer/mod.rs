pub mod hasher;
pub mod languages;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use walkdir::WalkDir;

use crate::storage::{IndexStats, SqliteStorage};
use languages::LanguageExtractor;

const DEFAULT_IGNORE: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "vendor",
    ".next",
    ".nuxt",
    "coverage",
    ".cache",
];

const DEFAULT_IGNORE_SUFFIXES: &[&str] = &[".min.js", ".min.mjs", ".min.cjs", ".min.css"];

#[derive(Debug, Clone)]
pub struct ExtractedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<ExtractedSymbol>,
    pub references: Vec<ExtractedRef>,
}

#[derive(Debug, Clone)]
pub struct ExtractedRef {
    pub name: String,
    pub kind: RefKind,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants are used by language extractors added in later phases
pub enum SymbolKind {
    Fn,
    Method,
    Class,
    Interface,
    Type,
    Struct,
    Enum,
    Trait,
    Mod,
    Const,
    Var,
    Impl,
    Section,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Method => "method",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Type => "type",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Mod => "mod",
            Self::Const => "const",
            Self::Var => "var",
            Self::Impl => "impl",
            Self::Section => "section",
        }
    }
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants are used by language extractors added in later phases
pub enum RefKind {
    Call,
    Import,
    TypeRef,
    Extends,
    Implements,
}

impl RefKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Import => "import",
            Self::TypeRef => "type_ref",
            Self::Extends => "extends",
            Self::Implements => "implements",
        }
    }
}

impl std::fmt::Display for RefKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

struct ExistingFile {
    id: i64,
    content_hash: String,
}

enum FileResult {
    New { symbols: usize, refs: usize },
    Changed { symbols: usize, refs: usize },
    Unchanged,
    Skipped,
}

pub struct Indexer {
    extractors: Vec<Box<dyn LanguageExtractor>>,
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer {
    pub fn new() -> Self {
        Self {
            extractors: vec![
                Box::new(languages::typescript::TypeScriptExtractor),
                Box::new(languages::python::PythonExtractor),
                Box::new(languages::rust_lang::RustExtractor),
                Box::new(languages::ruby::RubyExtractor),
                Box::new(languages::markdown::MarkdownExtractor),
            ],
        }
    }

    fn get_extractor(&self, ext: &str) -> Option<&dyn LanguageExtractor> {
        self.extractors
            .iter()
            .find(|e| e.extensions().contains(&ext))
            .map(|e| e.as_ref())
    }

    pub fn index(
        &self,
        repo_path: &str,
        storage: &SqliteStorage,
        ignore_patterns: &[String],
        max_file_size: u64,
    ) -> Result<IndexStats> {
        let start = Instant::now();
        let abs_path = std::fs::canonicalize(repo_path)
            .with_context(|| format!("Invalid repository path: {}", repo_path))?;
        let abs_path_str = abs_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))?;

        let repo_id = storage.ensure_repo(abs_path_str)?;
        storage.begin_transaction()?;

        let mut existing_map = build_existing_file_map(storage, repo_id)?;

        let mut files_new = 0;
        let mut files_changed = 0;
        let mut files_unchanged = 0;
        let mut total_symbols = 0;
        let mut total_refs = 0;

        let mut parser = tree_sitter::Parser::new();

        for entry in WalkDir::new(&abs_path)
            .into_iter()
            .filter_entry(|e| !is_ignored(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let ext = match path.extension().and_then(|e| e.to_str()) {
                Some(e) => e,
                None => continue,
            };

            let extractor = match self.get_extractor(ext) {
                Some(e) => e,
                None => continue,
            };

            if let Ok(meta) = entry.metadata() {
                if meta.len() > max_file_size {
                    continue;
                }
            }

            let rel_path = path
                .strip_prefix(&abs_path)
                .unwrap_or(path)
                .to_str()
                .unwrap_or("")
                .to_string();

            if rel_path.is_empty() {
                continue;
            }

            if matches_ignore_pattern(&rel_path, ignore_patterns) {
                continue;
            }

            let previous_entry = existing_map.remove(&rel_path);

            match process_file(
                &rel_path,
                path,
                extractor,
                &previous_entry,
                &mut parser,
                storage,
                repo_id,
            )? {
                FileResult::New { symbols, refs } => {
                    files_new += 1;
                    total_symbols += symbols;
                    total_refs += refs;
                }
                FileResult::Changed { symbols, refs } => {
                    files_changed += 1;
                    total_symbols += symbols;
                    total_refs += refs;
                }
                FileResult::Unchanged => {
                    files_unchanged += 1;
                }
                FileResult::Skipped => {}
            }
        }

        let files_deleted = remove_deleted_files(storage, &existing_map)?;

        storage.resolve_references(repo_id)?;
        storage.update_repo_timestamp(repo_id)?;
        storage.commit()?;

        Ok(IndexStats {
            files_total: files_new + files_changed + files_unchanged,
            files_new,
            files_changed,
            files_unchanged,
            files_deleted,
            symbols_count: total_symbols,
            refs_count: total_refs,
            duration_ms: start.elapsed().as_millis(),
        })
    }

    pub fn update_files(
        &self,
        repo_path: &str,
        files: &[String],
        storage: &SqliteStorage,
        ignore_patterns: &[String],
        max_file_size: u64,
    ) -> Result<IndexStats> {
        let start = Instant::now();
        let abs_path = std::fs::canonicalize(repo_path)?;
        let abs_path_str = abs_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path not UTF-8"))?;
        let repo_id = storage.ensure_repo(abs_path_str)?;
        storage.begin_transaction()?;

        let mut parser = tree_sitter::Parser::new();
        let mut total_symbols = 0;
        let mut total_refs = 0;
        let mut files_updated = 0;

        for rel_path in files {
            if matches_ignore_pattern(rel_path, ignore_patterns) {
                continue;
            }

            let full_path = abs_path.join(rel_path);
            if !full_path.exists() {
                continue;
            }

            if let Ok(meta) = std::fs::metadata(&full_path) {
                if meta.len() > max_file_size {
                    continue;
                }
            }

            let ext = match full_path.extension().and_then(|e| e.to_str()) {
                Some(e) => e,
                None => continue,
            };

            let extractor = match self.get_extractor(ext) {
                Some(e) => e,
                None => continue,
            };

            match process_file(
                rel_path,
                &full_path,
                extractor,
                &None,
                &mut parser,
                storage,
                repo_id,
            )? {
                FileResult::New { symbols, refs } | FileResult::Changed { symbols, refs } => {
                    files_updated += 1;
                    total_symbols += symbols;
                    total_refs += refs;
                }
                FileResult::Unchanged | FileResult::Skipped => {}
            }
        }

        storage.resolve_references(repo_id)?;
        storage.commit()?;

        Ok(IndexStats {
            files_total: files_updated,
            files_new: 0,
            files_changed: files_updated,
            files_unchanged: 0,
            files_deleted: 0,
            symbols_count: total_symbols,
            refs_count: total_refs,
            duration_ms: start.elapsed().as_millis(),
        })
    }
}

fn build_existing_file_map(
    storage: &SqliteStorage,
    repo_id: i64,
) -> Result<HashMap<String, ExistingFile>> {
    let existing_files = storage.get_files(repo_id)?;
    Ok(existing_files
        .into_iter()
        .map(|f| {
            (
                f.rel_path.clone(),
                ExistingFile {
                    id: f.id,
                    content_hash: f.content_hash,
                },
            )
        })
        .collect())
}

fn process_file(
    rel_path: &str,
    full_path: &Path,
    extractor: &dyn LanguageExtractor,
    previous_entry: &Option<ExistingFile>,
    parser: &mut tree_sitter::Parser,
    storage: &SqliteStorage,
    repo_id: i64,
) -> Result<FileResult> {
    let source = match std::fs::read(full_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(file = %rel_path, error = %e, "Failed to read file");
            return Ok(FileResult::Skipped);
        }
    };

    let hash = hasher::hash_bytes(&source);

    if let Some(existing) = previous_entry {
        if existing.content_hash == hash {
            return Ok(FileResult::Unchanged);
        }
    }

    if let Err(err) = parser.set_language(&extractor.language()) {
        tracing::warn!(file = %rel_path, error = %err, "Failed to set parser language");
        return Ok(FileResult::Skipped);
    }

    let tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => {
            tracing::warn!(file = %rel_path, "Failed to parse file");
            return Ok(FileResult::Skipped);
        }
    };

    let symbols = extractor.extract(&source, &tree);

    let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let language = languages::detect_language(ext).unwrap_or("unknown");
    let file_id = storage.upsert_file(repo_id, rel_path, &hash, language)?;
    storage.clear_file_symbols(file_id)?;

    let mut sym_count = 0;
    let mut ref_count = 0;
    for sym in &symbols {
        storage.insert_symbol_tree(file_id, repo_id, rel_path, sym, None)?;
        sym_count += count_symbols(sym);
        ref_count += count_refs(sym);
    }

    let is_new = previous_entry.is_none();
    if is_new {
        Ok(FileResult::New {
            symbols: sym_count,
            refs: ref_count,
        })
    } else {
        Ok(FileResult::Changed {
            symbols: sym_count,
            refs: ref_count,
        })
    }
}

fn remove_deleted_files(
    storage: &SqliteStorage,
    remaining: &HashMap<String, ExistingFile>,
) -> Result<usize> {
    let count = remaining.len();
    for existing in remaining.values() {
        storage.delete_file(existing.id)?;
    }
    Ok(count)
}

fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_str().unwrap_or("");
    if entry.file_type().is_dir() {
        return DEFAULT_IGNORE.contains(&name);
    }
    if entry.file_type().is_file() {
        return DEFAULT_IGNORE_SUFFIXES
            .iter()
            .any(|suffix| name.ends_with(suffix));
    }
    false
}

fn matches_ignore_pattern(rel_path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        if let Some(suffix) = pattern.strip_prefix('*') {
            // e.g. "*.min.js" → suffix match ".min.js"
            rel_path.ends_with(suffix)
        } else if pattern.ends_with('/') {
            // e.g. "generated/" → directory prefix match
            rel_path.starts_with(pattern) || rel_path.contains(&format!("/{}", pattern))
        } else {
            // Exact filename match
            rel_path == pattern || rel_path.ends_with(&format!("/{}", pattern))
        }
    })
}

fn count_symbols(sym: &ExtractedSymbol) -> usize {
    1 + sym.children.iter().map(count_symbols).sum::<usize>()
}

fn count_refs(sym: &ExtractedSymbol) -> usize {
    sym.references.len() + sym.children.iter().map(count_refs).sum::<usize>()
}
