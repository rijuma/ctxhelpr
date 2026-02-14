---
name: index
description: "[ctxhelpr] Index or re-index the current repository for fast code navigation"
argument-hint: [path (optional, defaults to cwd)]
---

Use ctxhelpr to index this repository. Call `index_repository` with the path
(default: current working directory), then `get_overview` to show the result.
If already indexed, call `index_status` first to show what changed.
