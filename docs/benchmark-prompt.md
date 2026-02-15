You are a benchmarking agent. Your job is to complete a series of code navigation
tasks on this repository using TWO approaches, then produce a report comparing them.

## Rules

1. **Do NOT use plan mode.** Execute tasks directly.
2. For each task, run it TWICE:
   - **Approach A ("native"):** Use ONLY Grep, Glob, and Read tools. Do NOT use any
     mcp__ctxhelpr__* tool. Pretend ctxhelpr does not exist.
   - **Approach B ("ctxhelpr"):** Use ONLY mcp__ctxhelpr__* tools. Do NOT use Grep,
     Glob, or Read (except for reading non-code files if needed).
3. For each approach on each task, track:
   - **Tool calls**: Count every tool invocation (e.g., 3x Grep + 2x Read = 5 calls)
   - **Wall time**: Wrap each approach in `date +%s%N` before and after, compute
     elapsed milliseconds using Bash
   - **Answer correctness**: Did both approaches reach the same correct answer?
4. Between Approach A and Approach B for the same task, do NOT reuse any information.
   Treat each approach as if you have zero prior knowledge of the codebase.
5. Before starting, run: `mcp__ctxhelpr__index_repository` on this repo (don't count
   this in the benchmark — it's setup). Record the indexing time separately.
6. Run `ctxhelpr --version` and record the output.
7. Record the current timestamp (`date -u +%Y-%m-%dT%H:%M:%SZ`) for the report header.
8. Collect environment info using Bash: OS (`uname -srm`), available memory (`free -h | head -2` on Linux or `sysctl hw.memsize` on macOS), and Claude model (check which model is powering this session — it's shown in the system prompt or available via internal metadata). Record all of this in the report header.
9. Write ALL results to `ctxhelpr-benchmark.md` at the repo root when done.

## Task Catalogue

Run ALL of the following tasks. Each must be completed to a definitive answer.

### T1 — Orientation: "What languages and top-level modules does this repo have?"
Goal: List every language detected, directory structure, and the 5 largest
types/classes by line count.

### T2 — Symbol lookup: "Find the function/method named `<X>` and return its full signature and doc comment."
Pick `<X>` = the first public function you find that has a doc comment (discover it
during Approach A, then use the same name for Approach B).

### T3 — Caller tracing: "Who calls `<X>`?"
Using the same symbol from T2, find every call site across the codebase. List each
caller's file, function name, and line number.

### T4 — Dependency tracing: "What does `<X>` depend on?"
For the same symbol, list every function/type/module it calls or references.

### T5 — Cross-file type usage: "Find all usages of the largest struct/class/interface."
Identify the largest type by line span, then find every file and function that
references it.

### T6 — Search by concept: "Find all symbols related to `<keyword>`."
Pick a domain keyword relevant to this repo (e.g., "auth", "parse", "index",
"config", "route"). Search for all symbols whose name contains or relates to it.
List name, kind, file, and line range for each.

### T7 — File structure: "List all symbols in the largest source file."
Find the source file with the most symbols. List every function, type, constant,
and import with their line ranges.

### T8 — Multi-hop trace: "Starting from the main entry point, trace 3 levels of calls."
Find the main/entry function, list what it calls (level 1), what those call
(level 2), and one more level (level 3). Produce a call tree.

### T9 — Change impact: "If I renamed `<Y>`, what files would break?"
Pick `<Y>` = a type or function that is referenced in at least 3 files. List every
file and symbol that would need updating.

### T10 — Documentation gap: "Find the 5 largest functions/methods that lack doc comments."
Search for functions above 10 lines that have no doc comment. List them with file,
name, line range, and size.

## Output Format

Write `./ctxhelpr-benchmark.md` with this exact structure:

````markdown
# ctxhelpr Benchmark Report

**Repository:** <repo name or anonymized identifier>
**Timestamp:** <ISO 8601 UTC, e.g. 2026-02-15T14:30:00Z>
**ctxhelpr version:** <output of ctxhelpr --version>
**Claude model:** <model name, e.g. claude-opus-4-6>
**OS:** <output of uname -srm>
**Memory:** <total available>
**Index time:** <ms>
**Total files indexed:** <N>
**Total symbols indexed:** <N>

## Summary Table

| Task | Native calls | Native ms | ctxhelpr calls | ctxhelpr ms | Speedup | Correct |
|------|-------------|-----------|----------------|-------------|---------|---------|
| T1   |             |           |                |             |         | Y/N     |
| T2   |             |           |                |             |         |         |
| ...  |             |           |                |             |         |         |

**Average speedup:** X.Xx
**Average call reduction:** X.Xx

## Detailed Results

### T1 — Orientation
#### Approach A (native)
- Tool calls: <list each tool and args>
- Wall time: <ms>
- Answer: <the answer>

#### Approach B (ctxhelpr)
- Tool calls: <list each tool and args>
- Wall time: <ms>
- Answer: <the answer>

#### Comparison
<1-2 sentence comparison>

<!-- repeat for T2-T10 -->

## Conclusions

<3-5 sentences on when ctxhelpr helps most, when native is comparable, and overall recommendation>
````

## Execution

Start now. Work through T1-T10 sequentially. For each task, complete Approach A
fully, then Approach B fully, then move to the next task. Do not parallelize
approaches within the same task (to avoid cross-contamination). You MAY parallelize
independent Bash timing calls.

Write the final `ctxhelpr-benchmark.md` when all 10 tasks are done.
