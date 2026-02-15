# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

## What This Is

ctxhelpr is an MCP server that semantically indexes codebases using tree-sitter, stores symbols/references in SQLite with FTS5, and exposes 10 tools for Claude Code (Initially) to navigate code structurally instead of reading raw files. Written in Rust.

If successful, this project will extend to other coding agents.

# Code Principles

- We prefer simple, clean maintainable solutions over clever or complex ones.
- Readability and maintainability are primary concerns.
- Self-documenting names and code. Only use additional comments when necessary.
- Small functions.
- Follow single responsibility principle in classes and functions.
- Code coverage is paramount.

## Commands

```text
cargo build --release          # Build (uses bundled SQLite via rusqlite)
cargo test                     # Run all tests (unit + integration)
cargo test test_name           # Run a single test
cargo test test_name -- --nocapture  # Run with stdout/stderr visible
RUST_LOG=ctxhelpr=debug cargo run -- serve   # Run MCP server with debug logging
```

ctxhelpr has eight subcommands: `serve` (MCP stdio server), `enable` (register with Claude Code), `disable` (remove integration), `perms` (manage tool permissions), `config` (project configuration), `repos` (manage indexed repositories), `update` (self-update to latest version), `uninstall` (completely remove ctxhelpr).

## Architecture

**Data flow:** Files on disk → tree-sitter parsing → `ExtractedSymbol`/`ExtractedRef` → SQLite storage → compact JSON output via MCP tools.

Key modules:

- **`mcp/`** — `CtxhelprServer` implements `ServerHandler` via rmcp macros (`#[tool_router]`, `#[tool_handler]`, `#[tool]`). Each MCP tool is a method. All tools take a repo path and open storage on demand. All handlers log at `tracing::info!` on entry with relevant parameters.
- **`indexer/`** — `Indexer` walks the repo, delegates to language extractors via the `LanguageExtractor` trait, handles incremental re-indexing via SHA256 content hashing. `ExtractedSymbol` trees are recursive (children + references).
- **`indexer/languages/`** — One module per language (TypeScript, Python, Rust, Ruby, Markdown). Each extractor returns `Vec<ExtractedSymbol>` from tree-sitter AST traversal.
- **`storage/`** — `SqliteStorage` wraps rusqlite. Schema is in `schema.sql` (loaded via `include_str!`). DB is per-repo, stored at `~/.cache/ctxhelpr/<hash>.db`. FTS5 virtual table with triggers keeps full-text index in sync. Provides `begin_transaction()`/`commit()` for batching — the indexer wraps all operations in a single transaction for performance.
- **`output/`** — `CompactFormatter` produces token-efficient JSON with short keys (`n`, `k`, `f`, `l`, `sig`, `doc`, `id`).
- **`cli/`** — `enable.rs` registers the MCP server, installs a skill file and `/reindex` command into `~/.claude/`. `disable.rs` reverses this.
- **`skills.rs`** — Shared constants (`SKILL_CONTENT`, `REINDEX_COMMAND_CONTENT`) and `refresh()` function for updating installed skill and command files. Used by `cli/update.rs`, `cli/enable.rs`, `mcp/`, and `watcher/`.
- **`watcher/`** — Background file watcher. On server startup, reindexes all known repos (blocking) and refreshes their skill files, then watches for filesystem changes via `notify` and triggers incremental reindex through a debouncer.
- **`assets/`** — Embedded markdown templates for the skill and slash command (included at compile time).

The `lib.rs` re-exports `config`, `indexer`, `output`, `storage`, `skills`, and `watcher` for use in integration tests.

## Adding a New Language Extractor

1. Create `src/indexer/languages/<lang>.rs` implementing `LanguageExtractor`
2. Register it in `src/indexer/languages/mod.rs` (add to `detect_language` match)
3. Add the extractor instance in `Indexer::new()` (`src/indexer/mod.rs`)
4. Add test fixtures under `tests/fixtures/<lang>/`

## Formatting & Linting

After making code changes, always run these checks and fix any issues before considering the task done:

1. `cargo fmt --all -- --check` — fix with `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings` — fix all warnings

## Testing

Integration tests in `tests/integration.rs` use `SqliteStorage::open_memory()` and index fixture files under `tests/fixtures/`. Tests cover: indexing, incremental re-index, symbol extraction (functions, classes, interfaces, enums, arrow functions), doc comments, call references, search, and compact output format.

## Rust Edition

Uses Rust edition 2024 (`edition = "2024"` in Cargo.toml), requiring rustc 1.85+.

## Documentation

All documentation files have English (`.md`) and Spanish (`.es.md`) versions.
When updating any documentation file, update both language versions with the
same structural and content changes. English is the source of truth.

Documentation structure:

- `README.md` / `README.es.md` — Project overview and quick start
- `docs/user-guide.md` / `docs/user-guide.es.md` — Configuration, tools reference, CLI details
- `docs/developer-guide.md` / `docs/developer-guide.es.md` — Building, architecture, contributing
- `docs/indexing-strategy.md` / `docs/indexing-strategy.es.md` — Indexing architecture deep dive
- `docs/benchmark-instructions.md` / `docs/benchmark-instructions.es.md` — Benchmarking methodology
- `docs/benchmark-prompt.md` — Standard benchmark prompt
