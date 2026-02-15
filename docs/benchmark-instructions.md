[![en](https://img.shields.io/badge/lang-en-green.svg)](./benchmark-instructions.md)
[![es](https://img.shields.io/badge/lang-es-lightgray.svg)](./benchmark-instructions.es.md)

# Benchmarking ctxhelpr

This benchmark runs a standardized comparison of ctxhelpr against native tools (Grep, Glob, Read) across 10 code navigation tasks. It produces a `ctxhelpr-benchmark.md` report with timing, tool call counts, and correctness for each task.

These results help us understand where ctxhelpr adds value and where it still falls short.

## Privacy

Before sharing results, please review them for sensitive information:

- Replace proprietary repository names with generic identifiers (e.g., "my-app")
- Redact proprietary symbol names, file paths, or internal domain terms
- Do not include code snippets from private repositories

## How to run

Make sure ctxhelpr is up to date and enabled for the repository you want to benchmark:

```text
ctxhelpr update          # update to the latest version
ctxhelpr enable          # enable for Claude Code
```

See the [Claude Code integration](./user-guide.md#claude-code-integration) section for more details.

Then copy the contents of [benchmark-prompt.md](./benchmark-prompt.md) into Claude Code on that repository.

## Results

Once the run completes, you'll find `ctxhelpr-benchmark.md` at the root of your repository. If you'd like to share it, send it to **[marcos@rigoli.dev](mailto:marcos@rigoli.dev)** — these reports directly inform development priorities.
