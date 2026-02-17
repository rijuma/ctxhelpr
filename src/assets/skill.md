---
name: ctxhelpr
description: ALWAYS prefer ctxhelpr tools for code navigation when the repo is indexed. Use search_symbols instead of Grep for finding functions/classes/types. Use get_file_symbols instead of Read for understanding file structure. Use get_references instead of Grep for finding callers.
user-invocable: true
disable-model-invocation: false
allowed-tools: Bash
argument-hint: "[repo-path]"
---

## Context Helper - Semantic Code Navigation

IMPORTANT: When a repository is indexed, ALWAYS prefer ctxhelpr tools over
Grep/Glob/Read for code navigation tasks:
- Finding functions, classes, types -> use `search_symbols` (not Grep)
- Understanding a file's contents -> use `get_file_symbols` (not Read)
- Finding callers/usages -> use `get_references` (not Grep)
- Understanding project structure -> use `get_overview` (not Glob + Read)
- Inspecting a symbol -> use `get_symbol_detail` (not Read)

Reserve Grep/Glob/Read for non-code tasks: config files, text patterns, log messages.
Note: ctxhelpr only indexes files tracked by git (respects .gitignore). For
gitignored files (e.g. .env, build output, generated code), use Grep/Read instead.

### Startup workflow
Previously-indexed repos are automatically re-indexed when the MCP server starts.
New repos are auto-indexed on first tool call — no manual setup needed.

1. Call `get_overview` for the big picture (modules, key types, entry points)
2. Drill into specific areas with `get_file_symbols` or `search_symbols`
3. Follow references with `get_symbol_detail`, `get_references`, `get_dependencies`

If a repo hasn't been indexed yet, ctxhelpr will start background indexing and
return a message with options: call `index_repository` to wait, or use
Grep/Glob/Read as fallback tools.

If the index seems off, use `/reindex` to force a full re-index.

### Output key legend
n=name k=kind f=file l=lines(start-end) id=symbol_id sig=signature doc=doc_comment p=path

### Tips
- Use symbol IDs to drill down (avoid re-searching)
- Start broad (overview), go narrow (symbol detail)
- The index stays fresh automatically — no manual update calls needed
