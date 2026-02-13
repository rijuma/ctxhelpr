[![en](https://img.shields.io/badge/lang-en-green.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## **Semantic code indexing for Claude Code**

Every time you start a new Claude Code session, it has to re-discover your entire codebase from scratch. That's slow, expensive, and lossy. **ctxhelpr** tries to mitigate that.

It's an [MCP](https://modelcontextprotocol.io) server that pre-indexes your repository semantically - functions, classes, types, references, call chains - and stores everything in a local SQLite database. Claude Code can then navigate your codebase through a set of targeted tools instead of dumping thousands of lines of raw code into context.

The result: faster context building, fewer tokens burned, and Claude actually _understands_ the structure of your code before touching it.

## Disclaimer

> [!WARNING]
> This project is **experimental** and under active development. It has not been thoroughly tested across diverse codebases, and there is no guarantee that the semantically indexed context it provides is more effective than the context a coding agent builds on its own. Use at your own risk.

If you encounter issues, have suggestions, or want to share your experience, please [open an issue](https://github.com/rijuma/ctxhelpr/issues) or reach out at [marcos@rigoli.dev](mailto:marcos@rigoli.dev).

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

- **TypeScript / TSX / JavaScript / JSX** - functions, classes, interfaces, enums, arrow functions, call references
- **Python** - functions, classes, inheritance, decorators, docstrings, constants
- **Rust** - functions, structs, enums, traits, impl blocks, modules, type aliases, constants
- **Ruby** - classes, modules, methods, singleton methods, inheritance, constants
- **Markdown** - heading hierarchy as sections with parent-child relationships

## Getting started

### Download

Download the latest release for your platform from the [releases page](https://github.com/rijuma/ctxhelpr/releases/latest).

| OS      | Architecture  | Asset                           |
| ------- | ------------- | ------------------------------- |
| Linux   | x86_64        | `ctxhelpr-*-linux-x64.tar.gz`   |
| Linux   | ARM64         | `ctxhelpr-*-linux-arm64.tar.gz` |
| macOS   | Apple Silicon | `ctxhelpr-*-macos-arm64.tar.gz` |
| macOS   | Intel         | `ctxhelpr-*-macos-x64.tar.gz`   |
| Windows | x86_64        | `ctxhelpr-*-windows-x64.zip`    |

### Install the binary

**Linux / macOS:**

```bash
tar xzf ctxhelpr-*.tar.gz
sudo mv ctxhelpr /usr/local/bin/
chmod +x /usr/local/bin/ctxhelpr
```

**Windows:**

Extract the `.zip` file and place `ctxhelpr.exe` in a directory that is in your `PATH`.

### Set up Claude Code integration

```bash
ctxhelpr install [-l | -g]
```

Registers the MCP server, installs the skill file and `/index` command, prompts to grant tool permissions, and prints the database path. Use `-l` / `--local` for the project's `.claude/` directory, or `-g` / `--global` for `~/.claude/`. If neither is specified, you'll be prompted to choose.

### Uninstall

```bash
ctxhelpr uninstall [-l | -g]
```

Removes all integrations and revokes tool permissions.

### Manage permissions

```bash
ctxhelpr perms [-l | -g] [-a | -r]
```

Manages which ctxhelpr tools Claude Code can call without prompting. Without flags, opens an interactive checklist. `-a` / `--all` grants all permissions; `-r` / `--remove` revokes them. During install you'll be asked to grant all; use `ctxhelpr perms` to change them later.

### Package managers

> [!NOTE]
> Distribution via package managers (brew, apt, npm/pnpm, etc.) is planned. For now, download the pre-built binary from the releases page.

## Configuration

### Project configuration (`.ctxhelpr.json`)

Place a `.ctxhelpr.json` file in your repository root to customize behavior per-project. All fields are optional and fall back to sensible defaults.

```json
{
  "output": {
    "max_tokens": 2000,
    "truncate_signatures": 120,
    "truncate_doc_comments": 100
  },
  "search": {
    "max_results": 20
  },
  "indexer": {
    "ignore": ["generated/", "*.min.js"],
    "max_file_size": 1048576
  }
}
```

### Configuration CLI

```bash
ctxhelpr config init                  # Create a .ctxhelpr.json template in the current directory
ctxhelpr config validate [--path dir] # Validate .ctxhelpr.json (check syntax and schema)
ctxhelpr config show [--path dir]     # Show resolved config (defaults merged with overrides)
```

### Field reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `output.max_tokens` | number or null | `null` | Limit response size (approximate, 1 token ~ 4 bytes) |
| `output.truncate_signatures` | number | `120` | Max signature length before truncation |
| `output.truncate_doc_comments` | number | `100` | Max doc comment length in brief views |
| `search.max_results` | number | `20` | Max search results returned |
| `indexer.ignore` | string[] | `[]` | Additional glob patterns of paths to ignore |
| `indexer.max_file_size` | number | `1048576` | Skip files larger than this (bytes) |

### Environment variables

| Variable   | Default | Description                       |
| ---------- | ------- | --------------------------------- |
| `RUST_LOG` | -       | Log level (e.g. `ctxhelpr=debug`) |

### Token budgeting

Responses can be constrained with `max_tokens` - either per-project in `.ctxhelpr.json` or per-request via the MCP tool parameter. When a response exceeds the budget, results are progressively truncated with a `"truncated": true` marker.

### Code-aware search

Search understands code naming conventions. Searching for `"user"` finds `getUserById`, `UserRepository`, and `user_service`. This works via pre-tokenized identifiers that split camelCase, PascalCase, and snake_case at word boundaries.

## How Claude uses it

Once set up, the workflow is transparent:

1. Claude detects you're working on code
2. Checks if the repo is indexed (`index_status`)
3. Gets a structural overview (`get_overview`)
4. Drills into specific areas as needed (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Follows call chains and dependencies (`get_references`, `get_dependencies`)
6. After you edit files, keeps the index fresh (`update_files`)

All of this happens automatically through the skill file - you don't need to do anything special.

## CLI reference

```bash
ctxhelpr                                    # Show help
ctxhelpr serve                              # MCP server (used internally by Claude Code)
ctxhelpr install [-l | -g]                  # Install integration
ctxhelpr uninstall [-l | -g]                # Remove integration
ctxhelpr perms [-l | -g] [-a | -r]          # Manage permissions
ctxhelpr config init                        # Create .ctxhelpr.json template
ctxhelpr config validate [--path dir]       # Validate config file
ctxhelpr config show [--path dir]           # Show resolved config
```

`serve` is not meant to be run manually. Claude Code spawns it via stdio; it stops automatically when the session ends.

When neither `-l` nor `-g` is specified: `install` prompts you to choose; other commands auto-detect by checking for a local `.claude/settings.json` first, falling back to global.

## Development

For contributors who want to build from source or work on ctxhelpr.

### Prerequisites

Requires Rust 1.85+ (edition 2024). If you're on an older version:

```bash
rustup update stable
```

### Build from source

```bash
cargo build --release
```

### Project structure

```text
src/
├── main.rs                 # CLI entry point
├── config.rs               # Project configuration (.ctxhelpr.json)
├── cli/                    # install, uninstall, perms & permissions
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

For detailed documentation on the indexing architecture, see [docs/indexing-strategy.md](docs/indexing-strategy.md).

### Tech stack

- **Rust** (edition 2024) - because startup time and memory matter for a tool that runs alongside your editor
- **tree-sitter** - fast, reliable parsing across languages
- **SQLite + FTS5** - single-file database with full-text search, no external dependencies
- **rmcp** - official Rust SDK for the Model Context Protocol
- **tokio** - async runtime for the MCP server
