[![en](https://img.shields.io/badge/lang-en-green.svg)](./indexing-strategy.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./indexing-strategy.es.md)

# Indexing Strategy

This document explains how ctxhelpr indexes codebases, the design decisions behind the approach, and known tradeoffs.

## Overview

ctxhelpr uses **tree-sitter** to parse source files into concrete syntax trees (CSTs), then extracts structural symbols (functions, classes, interfaces, etc.) and their relationships (calls, imports, type references). These are stored in **SQLite** with **FTS5** full-text search and served to AI agents via MCP tools.

The design premise: AI agents don't need to read raw source files to navigate code — structured, token-efficient summaries they can drill into on demand should be sufficient.

## Data Flow

```text
Files on disk
    |
    v
tree-sitter parsing (per-language grammars)
    |
    v
ExtractedSymbol / ExtractedRef (recursive trees)
    |
    v
SQLite storage (symbols, refs, FTS5 index)
    |
    v
Compact JSON output via MCP tools
```

## Incremental Indexing

### How It Works

1. **SHA-256 content hashing** - Each file's content is hashed at index time
2. **Hash comparison** - On re-index, existing hashes are compared to current content
3. **Selective re-parsing** - Only new or changed files are re-parsed
4. **Deleted file detection** - Files present in the DB but missing from disk are removed
5. **Single transaction** - All operations are wrapped in one SQLite transaction for atomicity

### Performance Characteristics

- **First index**: O(n) where n = total files. Tree-sitter parsing is fast (~1ms per file for most files)
- **Re-index (no changes)**: O(n) for the directory walk + O(m) for hash lookups, where m = indexed files. No parsing occurs.
- **Partial update** (`update_files`): O(k) where k = number of specified files. Bypasses directory walk entirely.
- **Transaction batching**: All inserts happen within a single `BEGIN IMMEDIATE`...`COMMIT`, avoiding per-row transaction overhead

### File Selection

Files are selected based on:

- **Extension mapping**: Each language extractor declares which extensions it handles (e.g., `.ts`, `.tsx`, `.js`, `.jsx` for TypeScript)
- **Size limit**: Files larger than 1 MiB (configurable via `.ctxhelpr.json`) are skipped
- **Gitignore support**: `.gitignore` files are respected automatically (including nested and global gitignore). Files ignored by git are skipped during indexing.
- **Default ignore patterns**: As a safety net (for repos without `.gitignore`), standard directories are also excluded: `node_modules`, `target`, `.git`, `dist`, `build`, `__pycache__`, `.venv`, `vendor`, `.next`, `.nuxt`, `coverage`, `.cache`
- **User config patterns**: Additional ignore patterns can be configured via `.ctxhelpr.json` `indexer.ignore` — these are applied on top of `.gitignore` and the default list

## Symbol Extraction

### Language Extractors

Each language has a dedicated extractor implementing the `LanguageExtractor` trait:

| Language   | Extractor           | Extensions                       |
| ---------- | ------------------- | -------------------------------- |
| TypeScript | TypeScriptExtractor | .ts, .tsx, .js, .jsx, .mjs, .cjs |
| Python     | PythonExtractor     | .py, .pyi                        |
| Rust       | RustExtractor       | .rs                              |
| Ruby       | RubyExtractor       | .rb                              |
| Markdown   | MarkdownExtractor   | .md, .markdown                   |

### Symbol Kinds

- `fn` - Functions and standalone function declarations
- `method` - Methods within classes/impls
- `class` - Class declarations
- `interface` - Interface declarations (TypeScript)
- `struct` - Struct declarations (Rust)
- `enum` - Enum declarations
- `trait` - Trait declarations (Rust)
- `mod` - Module declarations (Rust, Ruby)
- `const` - Constants
- `var` - Variables and assignments
- `impl` - Implementation blocks (Rust)
- `section` - Document sections (Markdown headings)
- `type` - Type aliases

### Reference Kinds

- `call` - Function/method calls
- `import` - Import statements
- `type_ref` - Type references in signatures
- `extends` - Class/interface inheritance
- `implements` - Interface implementation

### Recursive Tree Structure

Symbols are extracted as recursive trees: a class contains methods, an interface contains fields, an enum contains variants. The `ExtractedSymbol` struct has `children` and `references` fields. Storage flattens these into rows with `parent_symbol_id` foreign keys.

## Full-Text Search (FTS5)

### Indexed Columns

The FTS5 virtual table indexes five columns:

1. `name` - Symbol name as-is
2. `doc_comment` - Documentation strings
3. `kind` - Symbol kind (fn, class, etc.)
4. `file_rel_path` - Relative file path
5. `name_tokens` - Pre-split identifier subwords

### Code-Aware Tokenization

The default FTS5 `unicode61` tokenizer treats `getUserById` as a single token, making it impossible to search for "user" and find it. ctxhelpr solves this with a **pre-tokenization** approach:

At insert time, each symbol name is split into subwords:

- `getUserById` -> `"get user by id getuserbyid"`
- `UserRepository` -> `"user repository userrepository"`
- `MAX_RETRIES` -> `"max retries max_retries"`
- `HTMLParser` -> `"html parser htmlparser"`

These tokens are stored in the `name_tokens` column and indexed by FTS5. The original lowercased name is appended so exact matches still work.

**Splitting rules:**

- camelCase boundaries: `getUser` -> `get`, `user`
- PascalCase boundaries: `UserRepo` -> `user`, `repo`
- Underscore/hyphen/dot separators: `user_repo` -> `user`, `repo`
- Acronym boundaries: `HTMLParser` -> `html`, `parser` (split when uppercase run meets lowercase)

### Search Capabilities

- **Prefix matching**: `repo*` finds `UserRepository`
- **Boolean operators**: `user AND NOT admin`
- **Subword matching**: `"user"` finds `getUserById`, `UserRepository`, `user_service`
- **Doc comment search**: Searches across documentation text
- **Ranked results**: FTS5 BM25 ranking, ordered by relevance

## Schema Migration

ctxhelpr handles schema evolution gracefully:

1. A `metadata` table stores the current schema version
2. On `open()`, the storage detects if the DB is pre-migration (missing `name_tokens` column)
3. If migration is needed:
   - `ALTER TABLE` adds the new column
   - Existing symbols are backfilled with computed `name_tokens`
   - FTS triggers and table are rebuilt
   - Schema version is updated
4. `CREATE TABLE IF NOT EXISTS` ensures idempotent schema application

## Output Optimization

### Compact JSON Keys

All output uses abbreviated keys to minimize token consumption:

- `n` = name, `k` = kind, `f` = file, `l` = lines, `id` = symbol ID
- `sig` = signature, `doc` = doc comment, `p` = path

### File Path Deduplication

In multi-result responses (search results, references), file paths are deduplicated:

```json
{"_f": ["src/a.rs", "src/b.rs"], "hits": [{"fi": 0, ...}, {"fi": 1, ...}]}
```

When all results share a single file, the path is inlined directly (no index overhead).

### Signature Normalization

Signatures are normalized to save tokens:

- Whitespace around `:`, `,`, and bracket openers is removed
- `(a: number, b: number): number` becomes `(a:number,b:number):number`
- Signatures longer than 120 characters (configurable via `output.truncate_signatures`) are truncated with `...`

### Doc Comment Truncation

In brief views (overview, search results, file symbols), doc comments are truncated (limit configurable via `output.truncate_doc_comments`):

- First sentence (ending with `. `) if under 100 characters (default)
- First line if under 100 characters
- Word-boundary truncation with `...` otherwise

Detail views (`get_symbol_detail`) return full, untruncated signatures and docs.

### Token Budgeting

Responses can be budget-constrained:

- Per-request via `max_tokens` parameter
- Per-project via `.ctxhelpr.json` `[output] max_tokens`
- Uses byte-length approximation: `max_bytes = max_tokens * 4` (Claude averages ~4 bytes/token)
- Progressive truncation: removes array items until the response fits, adds `"truncated": true` marker

## Storage Architecture

### Per-Repository Databases

Each repository gets its own SQLite database at `~/.cache/ctxhelpr/<sha256-prefix>.db`. This avoids cross-repo interference and makes cleanup simple. Indexed repos can be listed and deleted via `repos list` / `repos delete` CLI subcommands or the `list_repos` / `delete_repos` MCP tools. The `disable` command also deletes relevant index databases, and `uninstall` removes the entire cache directory.

### WAL Mode

SQLite is configured with `PRAGMA journal_mode=WAL` for concurrent read/write performance. This matters when the MCP server handles multiple tool calls in parallel.

### FTS5 Triggers

Three triggers keep the FTS5 index synchronized:

- `symbols_ai` (after insert): Adds new symbol to FTS
- `symbols_ad` (after delete): Removes symbol from FTS
- `symbols_au` (after update): Re-indexes symbol in FTS

This means FTS is always consistent with the symbols table without manual rebuilds.

## Advantages

1. **Fast incremental updates** - Only changed files are re-parsed. Hash-based change detection is reliable and fast.
2. **Token-efficient output** - Compact keys, path deduplication, and truncation can reduce AI context consumption by an estimated 30-60% compared to raw output.
3. **Code-aware search** - Pre-tokenized identifiers make substring searches work across naming conventions.
4. **Language-agnostic core** - Adding a new language requires only implementing `LanguageExtractor`. Storage, output, and search work unchanged.
5. **No external dependencies** - SQLite is bundled (via `rusqlite`), tree-sitter grammars are compiled in. Single binary with no runtime dependencies.
6. **Configurable per-project** - `.ctxhelpr.json` allows tuning for project-specific needs.

## Disadvantages

1. **Tree-sitter grammar limitations** - Some complex or dynamic language constructs may not parse correctly. Tree-sitter grammars are "best effort" for each language.
2. **No cross-file type inference** - References are resolved by name matching (`refs.to_name = symbols.name`). If two symbols share a name, the wrong one may be linked.
3. **No runtime/dynamic analysis** - The indexer only sees static source code. Dynamically generated symbols, metaprogramming, or runtime imports are invisible.
4. **Token budget is approximate** - The 4-bytes-per-token heuristic is a rough proxy. Actual Claude tokenization may differ by 10-20%.
5. **Single-threaded parsing** - File parsing is sequential within a transaction. Very large repos (100k+ files) may take several seconds on first index.

## Edge Cases

### Encoding Issues

- Files that aren't valid UTF-8 in their path are skipped (`to_str()` returns `None`)
- Binary files are read but typically produce no valid tree-sitter parse
- Signature and doc comment truncation is UTF-8 safe — truncation points are snapped to valid character boundaries to avoid panics on multi-byte characters (emoji, CJK, accented chars)

### Duplicate Symbol Names

- Reference resolution picks the first match (`LIMIT 1`). This is correct for most cases but may mislink in repos with many identically-named symbols across modules.

### Empty Files

- Files with no extractable symbols still get a `files` row (they're tracked for change detection) but produce zero symbol/ref rows.

### Very Long Signatures

- Signatures over the configured limit (default 120 characters) are truncated in brief views but preserved in full in detail views.

### Concurrent Access

- WAL mode allows concurrent reads during indexing. However, `BEGIN IMMEDIATE` serializes writes, so two simultaneous index operations on the same repo will block.

### Schema Upgrades

- Migration from v1 (no `name_tokens`) to v2 is automatic. Future schema changes should follow the same pattern: detect old schema, alter, backfill, update version.

### Symlinks

- The `ignore` crate (used for directory walking) does not follow symlinks by default, avoiding circular symlink issues.

### Large Monorepos

- The file size limit (default 1 MiB, configurable via `indexer.max_file_size`) prevents indexing minified bundles or large generated files
- The directory ignore list skips `node_modules`, `target`, etc.
- Custom ignore patterns can be configured via `.ctxhelpr.json`
