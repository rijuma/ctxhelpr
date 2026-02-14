[![en](https://img.shields.io/badge/lang-en-green.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## Semantic code indexing for Claude Code

An [MCP](https://modelcontextprotocol.io) server that pre-indexes your repository using [tree-sitter](https://tree-sitter.github.io/) - functions, classes, types, references, call chains - and stores everything in a local SQLite database. Claude Code navigates your codebase through targeted tools instead of reading thousands of lines of raw code.

The result: faster context building, fewer tokens burned, and Claude actually _understands_ the structure of your code before touching it.

> [!WARNING]
> This project is **experimental** and under active development. There is no guarantee that the indexed context is more effective than what a coding agent builds on its own. Use at your own risk.

## Getting started

```text
curl -sSfL https://sh.ctxhelpr.dev | sh
```

Then install it to Claude Code:

```text
ctxhelpr install
```

Run `ctxhelpr --help` for all CLI commands.

## Highlights

- **Incremental indexing** - SHA256 content hashing, only changed files are re-parsed
- **Code-aware search** - searching "user" finds `getUserById`, `UserRepository`, `user_service`
- **Token-efficient output** - compact keys, path deduplication, configurable budgets
- **11 MCP tools** for structural navigation

## Language support

- TypeScript / TSX / JavaScript / JSX
- Python
- Rust
- Ruby
- Markdown

## Documentation

- [User Guide](docs/user-guide.md) - installation, configuration, tools reference, CLI details
- [Developer Guide](docs/developer-guide.md) - building from source, architecture, contributing
- [Indexing Strategy](docs/indexing-strategy.md) - indexing architecture deep dive
- [Changelog](CHANGELOG.md)

All documentation is available in [Spanish](README.es.md) as well.

## License

[MIT](LICENSE)
