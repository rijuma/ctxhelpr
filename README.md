[![en](https://img.shields.io/badge/lang-en-green.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./README.es.md)

# ctxhelpr

**Semantic code indexing for Claude Code.**

Every time you start a new Claude Code session, it has to re-discover your entire codebase from scratch. That's slow, expensive, and lossy. **ctxhelpr** fixes that.

It's an [MCP](https://modelcontextprotocol.io/) server that pre-indexes your repository semantically - functions, classes, types, references, call chains - and stores everything in a local SQLite database. Claude Code can then navigate your codebase through a set of targeted tools instead of dumping thousands of lines of raw code into context.

The result: faster context building, fewer tokens burned, and Claude actually _understands_ the structure of your code before touching it.

## Status

**This is a proof of concept.** I built it to explore the idea and see if semantic indexing could meaningfully improve the Claude Code experience. It works, it's functional, but it's not battle-tested. Expect rough edges. If you find this useful or have ideas, I'd love to hear about it.

## How it works

1. **Indexes your repo** using [tree-sitter](https://tree-sitter.github.io/) to extract symbols, their relationships, and documentation
2. **Stores everything** in a per-repo SQLite database with FTS5 full-text search
3. **Exposes 9 MCP tools** that Claude Code uses to navigate your code semantically
4. **Incremental re-indexing** - only re-parses files that actually changed (SHA256 content hashing)

### MCP Tools

| Tool                | What it does                                             |
| ------------------- | -------------------------------------------------------- |
| `index_repository`  | Full index/re-index with incremental hash-checking       |
| `update_files`      | Fast re-index of specific files after edits (~50ms)      |
| `get_overview`      | High-level repo structure: languages, modules, key types |
| `get_file_symbols`  | All symbols in a file with signatures and line ranges    |
| `get_symbol_detail` | Full details: signature, docs, calls, callers, type refs |
| `search_symbols`    | Full-text search across symbol names and docs            |
| `get_references`    | Who references a given symbol                            |
| `get_dependencies`  | What a symbol depends on                                 |
| `index_status`      | Check index freshness and detect stale files             |

## Language support

Currently implemented:

- **TypeScript / TSX / JavaScript / JSX** - full extraction

Infrastructure is ready for Python and Rust, but extractors aren't written yet.

## Getting started

### Prerequisites

Requires Rust 1.85+ (edition 2024). If you're on an older version:

```bash
rustup update stable
```

### Build

```bash
cargo build --release
```

### Setup

```bash
ctxhelpr setup [-l | -g]
```

Registers the MCP server, installs the skill file and `/index` command, prompts to grant tool permissions, and prints the database path. Use `-l` / `--local` for the project's `.claude/` directory, or `-g` / `--global` for `~/.claude/`. If neither is specified, you'll be prompted to choose.

### Uninstall

```bash
ctxhelpr uninstall [-l | -g]
```

Removes all integrations and revokes tool permissions.

### Permissions

```bash
ctxhelpr perms [-l | -g] [-a | -r]
```

Manages which ctxhelpr tools Claude Code can call without prompting. Without flags, opens an interactive checklist. `-a` / `--all` grants all permissions; `-r` / `--remove` revokes them. During setup you'll be asked to grant all; use `ctxhelpr perms` to change them later.

### CLI

```bash
ctxhelpr                                    # Show help
ctxhelpr serve                              # MCP server (used internally by Claude Code)
ctxhelpr setup [-l | -g]                    # Set up integration
ctxhelpr uninstall [-l | -g]                # Remove integration
ctxhelpr perms [-l | -g] [-a | -r]          # Manage permissions
```

`serve` is not meant to be run manually. Claude Code spawns it via stdio; it stops automatically when the session ends.

When neither `-l` nor `-g` is specified: `setup` prompts you to choose; other commands auto-detect by checking for a local `.claude/settings.json` first, falling back to global.

## Configuration

All configuration is through environment variables - no config files needed.

| Variable                   | Default                             | Description                       |
| -------------------------- | ----------------------------------- | --------------------------------- |
| `RUST_LOG`                 | -                                   | Log level (e.g. `ctxhelpr=debug`) |
| `CTXHELPR_DB_DIR`          | `~/.cache/ctxhelpr/`                | Database storage location         |
| `CTXHELPR_MAX_FILE_SIZE`   | `1048576` (1MB)                     | Skip files larger than this       |
| `CTXHELPR_IGNORE_PATTERNS` | `node_modules,target,.git,dist,...` | Directories to skip               |

## How Claude uses it

Once set up, the workflow is transparent:

1. Claude detects you're working on code
2. Checks if the repo is indexed (`index_status`)
3. Gets a structural overview (`get_overview`)
4. Drills into specific areas as needed (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Follows call chains and dependencies (`get_references`, `get_dependencies`)
6. After you edit files, keeps the index fresh (`update_files`)

All of this happens automatically through the skill file - you don't need to do anything special.

## Tech stack

- **Rust** (edition 2024) - because startup time and memory matter for a tool that runs alongside your editor
- **tree-sitter** - fast, reliable parsing across languages
- **SQLite + FTS5** - single-file database with full-text search, no external dependencies
- **rmcp** - official Rust SDK for the Model Context Protocol
- **tokio** - async runtime for the MCP server

## Project structure

```text
src/
├── main.rs                 # CLI entry point
├── cli/                    # setup, uninstall, perms & permissions
├── server/                 # MCP server (stdio transport)
├── mcp/                    # Tool definitions and handlers
├── indexer/                # Core indexing logic + language extractors
│   └── languages/          # tree-sitter based extractors
├── storage/                # SQLite persistence + schema
├── output/                 # Token-efficient JSON formatting
└── assets/                 # Embedded skill & command templates
```
