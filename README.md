[![en](https://img.shields.io/badge/lang-en-green.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## Semantic code indexing for Claude Code

An [MCP](https://modelcontextprotocol.io) server that pre-indexes your repository using [tree-sitter](https://tree-sitter.github.io/) - functions, classes, types, references, call chains - and stores everything in a local SQLite database. Claude Code navigates your codebase through targeted tools instead of reading thousands of lines of raw code.

The goal: faster context building, lower token usage, and better structural awareness before Claude modifies your code.

> [!WARNING]
> This project is **experimental** and under active development. There is no guarantee that the indexed context is more effective than what a coding agent builds on its own. Use at your own risk.

## Getting started

```text
curl -sSfL https://sh.ctxhelpr.dev | sh
```

Then enable it in Claude Code:

```text
ctxhelpr enable
```

Run `ctxhelpr --help` for all CLI commands.

## Highlights

- **Incremental indexing** - SHA256 content hashing, only changed files are re-parsed
- **Code-aware search** - searching "user" finds `getUserById`, `UserRepository`, `user_service`
- **Token-efficient output** - compact keys, path deduplication, configurable budgets
- **11 MCP tools** for structural navigation

## Privacy

ctxhelpr runs entirely on your machine. Your code never leaves your local environment - all indexing, storage, and querying happens locally. The only external network access occurs when you explicitly run `ctxhelpr update` to check for new releases.

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

## Benchmarks

ctxhelpr is experimental and we need real-world data to understand where it helps and where it falls short. If you're willing, run the [benchmark instructions](docs/benchmark-instructions.md) on your repositories using Claude Code and send the resulting `ctxhelpr-benchmark.md` to [marcos@rigoli.dev](mailto:marcos@rigoli.dev) — it helps us focus on what matters.

## License

[MIT](LICENSE)
