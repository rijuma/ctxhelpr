---
name: ctxhelpr
description: Use at the start of sessions to quickly understand a codebase via semantic index. Use when exploring unfamiliar code, finding functions, or tracing call chains. Automatically detects if a repo is already indexed.
user-invocable: true
disable-model-invocation: false
allowed-tools: Bash
argument-hint: [repo-path]
---

## Context Helper - Semantic Code Navigation

When starting work on a codebase, use the ctxhelpr MCP tools to quickly build context:

### Startup workflow
1. Call `index_status` to check if the repo is indexed and fresh
2. If stale or unindexed, call `index_repository` first
3. Call `get_overview` for the big picture (modules, key types, entry points)
4. Drill into specific areas with `get_file_symbols` or `search_symbols`
5. Follow references with `get_symbol_detail`, `get_references`, `get_dependencies`

### Keep index fresh while coding
After completing edits to files, call `update_files` with the list of
files you just modified. This re-indexes only those files (~50ms) and
keeps the index current without a full repo walk. Do this after each
task or edit batch, not after every single line change.

### Output key legend
n=name k=kind f=file l=lines(start-end) id=symbol_id sig=signature doc=doc_comment p=path

### Tips
- Use symbol IDs to drill down (avoid re-searching)
- Start broad (overview), go narrow (symbol detail)
- Call `update_files` after edits to keep the index fresh
