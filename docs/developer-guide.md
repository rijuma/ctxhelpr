[![en](https://img.shields.io/badge/lang-en-green.svg)](./developer-guide.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./developer-guide.es.md)

# Developer Guide

[Back to README](../README.md)

## Prerequisites

Requires Rust 1.85+ (edition 2024). If you're on an older version:

```text
rustup update stable
```

## Build from Source

```text
cargo build --release
```

Uses bundled SQLite via rusqlite - no external dependencies needed.

## Running and Testing

### Commands

```text
cargo build --release                        # Build
cargo test                                   # Run all tests (unit + integration)
cargo test test_name                         # Run a single test
cargo test test_name -- --nocapture          # Run with stdout/stderr visible
RUST_LOG=ctxhelpr=debug cargo run -- serve   # Run MCP server with debug logging
```

ctxhelpr has eight subcommands: `serve`, `enable`, `disable`, `perms`, `config`, `repos`, `update`, `uninstall`.

### Testing

Integration tests in `tests/integration.rs` use `SqliteStorage::open_memory()` and index fixture files under `tests/fixtures/`. Tests cover: indexing, incremental re-index, symbol extraction (functions, classes, interfaces, enums, arrow functions), doc comments, call references, search, and compact output format.

### Formatting & Linting

After making code changes, always run these checks and fix any issues before considering the task done:

1. `cargo fmt --all -- --check` - fix with `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings` - fix all warnings

## Architecture

### Data flow

```text
Files on disk → tree-sitter parsing → ExtractedSymbol/ExtractedRef → SQLite storage → compact JSON output via MCP tools
```

### Project structure

```text
src/
├── main.rs                 # CLI entry point
├── config.rs               # Project configuration (.ctxhelpr.json)
├── cli/                    # enable, disable, perms, permissions & repos
├── server/                 # MCP server (stdio transport)
├── mcp/                    # Tool definitions and handlers
├── indexer/                # Core indexing logic + language extractors
│   └── languages/          # tree-sitter based extractors (TS, Python, Rust, Ruby, MD)
├── storage/                # SQLite persistence + schema + code tokenizer
├── output/                 # Token-efficient JSON formatting + budgeting
│   ├── formatter.rs        # OutputFormatter trait
│   └── token_budget.rs     # Token budget enforcement
└── assets/                 # Embedded skill & command templates
```

### Key modules

- **`mcp/`** - `CtxhelprServer` implements `ServerHandler` via rmcp macros (`#[tool_router]`, `#[tool_handler]`, `#[tool]`). Each MCP tool is a method. All tools take a repo path and open storage on demand. All handlers log at `tracing::info!` on entry with relevant parameters.
- **`indexer/`** - `Indexer` walks the repo using the `ignore` crate (respects `.gitignore`), delegates to language extractors via the `LanguageExtractor` trait, handles incremental re-indexing via SHA256 content hashing. `ExtractedSymbol` trees are recursive (children + references).
- **`indexer/languages/`** - One module per language (TypeScript, Python, Rust, Ruby, Markdown). Each extractor returns `Vec<ExtractedSymbol>` from tree-sitter AST traversal.
- **`storage/`** - `SqliteStorage` wraps rusqlite. Schema is in `schema.sql` (loaded via `include_str!`). DB is per-repo, stored at `~/.cache/ctxhelpr/<hash>.db`. FTS5 virtual table with triggers keeps full-text index in sync. Provides `begin_transaction()`/`commit()` for batching - the indexer wraps all operations in a single transaction for performance.
- **`output/`** - `CompactFormatter` produces token-efficient JSON with short keys (`n`, `k`, `f`, `l`, `sig`, `doc`, `id`).
- **`cli/`** - `enable.rs` registers the MCP server, installs a skill file and `/reindex` command into `~/.claude/`. `disable.rs` removes the registration, skill file, command, index databases, and project config.
- **`assets/`** - Embedded markdown templates for the skill and slash command (included at compile time).

The `lib.rs` re-exports `indexer`, `output`, and `storage` for use in integration tests.

### Tech stack

- **Rust** (edition 2024) - because startup time and memory matter for a tool that runs alongside your editor
- **tree-sitter** - fast, reliable parsing across languages
- **SQLite + FTS5** - single-file database with full-text search, no external dependencies
- **rmcp** - official Rust SDK for the Model Context Protocol
- **tokio** - async runtime for the MCP server

## Adding a New Language Extractor

1. Create `src/indexer/languages/<lang>.rs` implementing `LanguageExtractor`
2. Register it in `src/indexer/languages/mod.rs` (add to `detect_language` match)
3. Add the extractor instance in `Indexer::new()` (`src/indexer/mod.rs`)
4. Add test fixtures under `tests/fixtures/<lang>/`

## Code Principles

- We prefer simple, clean maintainable solutions over clever or complex ones.
- Readability and maintainability are primary concerns.
- Self-documenting names and code. Only use additional comments when necessary.
- Small functions.
- Follow single responsibility principle in classes and functions.
- Code coverage is paramount.

## Documentation

All documentation files have English (`.md`) and Spanish (`.es.md`) versions. When updating any documentation file, update both language versions with the same structural and content changes. English is the source of truth.

Documentation structure:

- `README.md` / `README.es.md` - Project overview and quick start
- `docs/user-guide.md` / `docs/user-guide.es.md` - Configuration, tools reference, CLI details
- `docs/developer-guide.md` / `docs/developer-guide.es.md` - Building, architecture, contributing
- `docs/indexing-strategy.md` / `docs/indexing-strategy.es.md` - Indexing architecture deep dive

## Further Reading

- [Indexing Strategy](./indexing-strategy.md) - deep dive into the indexing architecture
- [User Guide](./user-guide.md) - configuration, tools reference, CLI details
