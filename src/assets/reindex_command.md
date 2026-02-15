---
name: reindex
description: "(ctxhelpr) Force a full re-index of the current repository (rarely needed — ctxhelpr auto-indexes)"
argument-hint: [path (optional, defaults to cwd)]
---

Force ctxhelpr to fully re-index this repository. This is rarely needed since
ctxhelpr automatically watches for file changes and keeps the index fresh.

Use this when:

- The index seems out of date or incorrect
- You just switched branches and want to force a refresh
- Something seems off with search results

Call `index_repository` with the path (default: current working directory),
then `get_overview` to show the result.
