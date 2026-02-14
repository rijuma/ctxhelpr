[![en](https://img.shields.io/badge/lang-en-green.svg)](./user-guide.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./user-guide.es.md)

# User Guide

[Back to README](../README.md)

## Installation

### Quick install

```bash
curl -sSf https://sh.ctxhelpr.dev | sh
```

Detects your platform, downloads the latest release, verifies the checksum, and installs to `~/.local/bin/`.

Options:

```bash
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --version 1.1.0    # Specific version
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --install-dir DIR   # Custom directory
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --skip-setup        # Download only, no setup
```

### Manual install

Download the latest release for your platform from the [releases page](https://github.com/rijuma/ctxhelpr/releases/latest).

| OS      | Architecture  | Asset                           |
| ------- | ------------- | ------------------------------- |
| Linux   | x86_64        | `ctxhelpr-*-linux-x64.tar.gz`   |
| Linux   | ARM64         | `ctxhelpr-*-linux-arm64.tar.gz` |
| macOS   | Apple Silicon | `ctxhelpr-*-macos-arm64.tar.gz` |
| macOS   | Intel         | `ctxhelpr-*-macos-x64.tar.gz`   |
| Windows | x86_64        | `ctxhelpr-*-windows-x64.zip`    |

**Linux / macOS:**

```bash
tar xzf ctxhelpr-*.tar.gz
mv ctxhelpr ~/.local/bin/
```

**Windows:**

Extract the `.zip` file and place `ctxhelpr.exe` in a directory that is in your `PATH`.

### Package managers

> [!NOTE]
> Distribution via package managers (brew, apt, npm/pnpm, etc.) is planned. For now, download ctxhelpr from the releases page.

## Setup

### Claude Code integration

```bash
ctxhelpr install [-l | -g]
```

Registers the MCP server, installs the skill file and `/index` command, prompts to grant tool permissions, and prints the database path. Use `-l` / `--local` for the project's `.claude/` directory, or `-g` / `--global` for `~/.claude/`. If neither is specified, you'll be prompted to choose.

### Permissions management

```bash
ctxhelpr perms [-l | -g] [-a | -r]
```

Manages which ctxhelpr tools Claude Code can call without prompting. Without flags, opens an interactive checklist. `-a` / `--all` grants all permissions; `-r` / `--remove` revokes them. During install you'll be asked to grant all; use `ctxhelpr perms` to change them later.

### Uninstall

```bash
ctxhelpr uninstall [-l | -g]
```

Removes all integrations and revokes tool permissions. Prompts to delete index databases: local uninstall offers to delete the current repo's DB (default: yes), global uninstall offers to delete all DBs (default: no).

## MCP Tools Reference

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
| `list_repos`        | List all indexed repositories with stats                 |
| `delete_repos`      | Delete index data for specified repositories             |

## Language Support

- **TypeScript / TSX / JavaScript / JSX** - functions, classes, interfaces, enums, arrow functions, call references
- **Python** - functions, classes, inheritance, decorators, docstrings, constants
- **Rust** - functions, structs, enums, traits, impl blocks, modules, type aliases, constants
- **Ruby** - classes, modules, methods, singleton methods, inheritance, constants
- **Markdown** - heading hierarchy as sections with parent-child relationships

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

## Token Budgeting

Responses can be constrained with `max_tokens` - either per-project in `.ctxhelpr.json` or per-request via the MCP tool parameter. When a response exceeds the budget, results are progressively truncated with a `"truncated": true` marker.

## Code-Aware Search

Search understands code naming conventions. Searching for `"user"` finds `getUserById`, `UserRepository`, and `user_service`. This works via pre-tokenized identifiers that split camelCase, PascalCase, and snake_case at word boundaries.

## How Claude Uses It

Once set up, the workflow is transparent:

1. Claude detects you're working on code
2. Checks if the repo is indexed (`index_status`)
3. Gets a structural overview (`get_overview`)
4. Drills into specific areas as needed (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Follows call chains and dependencies (`get_references`, `get_dependencies`)
6. After you edit files, keeps the index fresh (`update_files`)

This all happens automatically via the skill file — no additional setup needed.

## CLI Reference

```bash
ctxhelpr                                    # Show help
ctxhelpr serve                              # MCP server (used internally by Claude Code)
ctxhelpr install [-l | -g]                  # Install integration
ctxhelpr uninstall [-l | -g]                # Remove integration
ctxhelpr perms [-l | -g] [-a | -r]          # Manage permissions
ctxhelpr config init                        # Create .ctxhelpr.json template
ctxhelpr config validate [--path dir]       # Validate config file
ctxhelpr config show [--path dir]           # Show resolved config
ctxhelpr repos list                         # List all indexed repositories
ctxhelpr repos delete [paths...]            # Delete index data (interactive if no paths)
```

`serve` is not meant to be run manually. Claude Code spawns it via stdio; it stops automatically when the session ends.

When neither `-l` nor `-g` is specified: `install` prompts you to choose; other commands auto-detect by checking for a local `.claude/settings.json` first, falling back to global.
