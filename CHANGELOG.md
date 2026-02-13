# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2026-02-13

### Bug Fixes

- Tag release fix ([573cf79](https://github.com/rijuma/ctxhelpr/commit/573cf792c5f5375e74f92d36c435995230edb9de))
- Fixed x86 apple build ([0d3393a](https://github.com/rijuma/ctxhelpr/commit/0d3393adeaa4c090a161d861f098f06dd045ce35))

### Features

- Dded Python, Rust, Ruby and Markdown parsing (#18)
## New Language Extractors

  Added semantic code indexing support for 4 new languages using
  tree-sitter:

  - Python — functions, classes, methods, inheritance, docstrings, type
  hints, constants
  - Rust — functions, structs, enums, traits, impl blocks, modules, type
  aliases, doc comments
  - Ruby — classes, modules, methods, singleton methods, constants,
  require/include
  - Markdown — heading hierarchy (H1–H6) as navigable document sections

  Each extractor parses source files into a tree of symbols with
  signatures, doc comments, and cross-references (calls,
  imports, extends), enabling structural code navigation through the MCP
  tools. A new Section symbol kind was added for
  markdown headings. ([1efdf19](https://github.com/rijuma/ctxhelpr/commit/1efdf191e3bd194c17ac19bd484908df3e00a7c5))

### Miscellaneous

- First commit ([9cc49db](https://github.com/rijuma/ctxhelpr/commit/9cc49dba961b19149f4c16725748ce2e34b02978))
- Fix tag and build ([c1efc91](https://github.com/rijuma/ctxhelpr/commit/c1efc9107951d60346c86982be445260198f03ef))
- Improved asset name for builds ([18d19fc](https://github.com/rijuma/ctxhelpr/commit/18d19fc595182199fd30f92d76ca47472edf65e9))
- Updated build jobs to match the new labels ([3a9f657](https://github.com/rijuma/ctxhelpr/commit/3a9f6579808653ee6c36ccd0f425be80f122584a))
- Updated docs ([6873a83](https://github.com/rijuma/ctxhelpr/commit/6873a832282a02e33a6f6843e4b04e357f313697))

### Other

- Add workflows (#1)

Added github workflows for CI ([948042a](https://github.com/rijuma/ctxhelpr/commit/948042ac797eb29044bd10e4a9d47c8cb3fb3a9f))
- Add local and global scopes and add default permission handling (#2)

- Added possibility to manage two setting scopes, local and global:
- The local scope allows to enable indexing, skill and permissions
locally.
- The global scope enables the indexing and the skill to Claude
globally.
- Added prompting to handle new options and flags to skip the prompting. ([d2c6633](https://github.com/rijuma/ctxhelpr/commit/d2c66336ea8218e42101780aead2250b368a5f25))
- Rename setup for install and added release workflows (#3)

- Renamed the `setup` command to `install`, to be more clear and aligned
with the opposite `uninstall` we are already using.
- Added changelog logic, tag actions and release build workflows.
- Updated docs. ([bb9490b](https://github.com/rijuma/ctxhelpr/commit/bb9490bdd684a2b9531a0a9c9bc0030b1774b9ec))
- Update workflows and docs (#4)

- Updated release workflow as follows:
1. Run the Publish workflow. This will bump the version (major, minor or
patch), update the changelog and create a PR with these changes. This
will make sure the CI passes and allows some manual tweaking if needed.
2. Once merged, it will create a tag, build the releases and create a
release with the available downloads.
3. The release is not set as "latest", so after confirming everything
works as intended, we should open the release and flag it as "latest".
- Updated documentation to be more user-oriented, rather than
collaborator-oriented. ([cf07395](https://github.com/rijuma/ctxhelpr/commit/cf0739588029b7098ec878d8f35262ea0a533c7c))
- Improve release workflow (#5)

- Split release workflow to be more efficient. ([0cf1993](https://github.com/rijuma/ctxhelpr/commit/0cf199347fa0aa45bbca1df8a68b3736399b582a))
- Fixed release workflow (#7)

- Updated PR content and fixed CI workflows never starting. ([7824095](https://github.com/rijuma/ctxhelpr/commit/7824095b540c06f6701327087a9fd2714f31b0af))
- Fix release workflow (#11)

## Summary
- Use REST API for PR creation to avoid `GITHUB_TOKEN` GraphQL
permission issue ("Resource not accessible by integration")
- Add `workflow_dispatch` trigger to CI so release workflow can trigger
it
- Add resilience for re-runs: delete pre-existing remote branch before
push
- Improve PR description with actual changelog content ([0e6fca6](https://github.com/rijuma/ctxhelpr/commit/0e6fca60c7856844b63de111b422be5f26345034))
- Updated release workflow to use personal tokens instead (#13)

Updated release workflow to use personal tokens instead ([07bc073](https://github.com/rijuma/ctxhelpr/commit/07bc073f41995adebdf7142e02628e110c63cd4f))
- Confirm new release v0.2.0 (#14)

## Release v0.2.0

Bumps version from 0.1.0 to 0.2.0 (minor).

### Changelog

## [0.2.0] - 2026-02-12

### Miscellaneous

- First commit
([9cc49db](https://github.com/rijuma/ctxhelpr/commit/9cc49dba961b19149f4c16725748ce2e34b02978))

### Other

- Add workflows (#1)

Added github workflows for CI

([948042a](https://github.com/rijuma/ctxhelpr/commit/948042ac797eb29044bd10e4a9d47c8cb3fb3a9f))
- Add local and global scopes and add default permission handling (#2)

- Added possibility to manage two setting scopes, local and global:
- The local scope allows to enable indexing, skill and permissions
locally.
- The global scope enables the indexing and the skill to Claude
globally.
- Added prompting to handle new options and flags to skip the prompting.

([d2c6633](https://github.com/rijuma/ctxhelpr/commit/d2c66336ea8218e42101780aead2250b368a5f25))
- Rename setup for install and added release workflows (#3)

- Renamed the `setup` command to `install`, to be more clear and aligned
with the opposite `uninstall` we are already using.
- Added changelog logic, tag actions and release build workflows.
- Updated docs.

([bb9490b](https://github.com/rijuma/ctxhelpr/commit/bb9490bdd684a2b9531a0a9c9bc0030b1774b9ec))
- Update workflows and docs (#4)

- Updated release workflow as follows:
1. Run the Publish workflow. This will bump the version (major, minor or
patch), update the changelog and create a PR with these changes. This
will make sure the CI passes and allows some manual tweaking if needed.
2. Once merged, it will create a tag, build the releases and create a
release with the available downloads.
3. The release is not set as "latest", so after confirming everything
works as intended, we should open the release and flag it as "latest".
- Updated documentation to be more user-oriented, rather than
collaborator-oriented.

([cf07395](https://github.com/rijuma/ctxhelpr/commit/cf0739588029b7098ec878d8f35262ea0a533c7c))
- Improve release workflow (#5)

- Split release workflow to be more efficient.

([0cf1993](https://github.com/rijuma/ctxhelpr/commit/0cf199347fa0aa45bbca1df8a68b3736399b582a))
- Fixed release workflow (#7)

- Updated PR content and fixed CI workflows never starting.

([7824095](https://github.com/rijuma/ctxhelpr/commit/7824095b540c06f6701327087a9fd2714f31b0af))
- Fix release workflow (#11)

## Summary
- Use REST API for PR creation to avoid `GITHUB_TOKEN` GraphQL
permission issue ("Resource not accessible by integration")
- Add `workflow_dispatch` trigger to CI so release workflow can trigger
it
- Add resilience for re-runs: delete pre-existing remote branch before
push
- Improve PR description with actual changelog content

([0e6fca6](https://github.com/rijuma/ctxhelpr/commit/0e6fca60c7856844b63de111b422be5f26345034))
- Updated release workflow to use personal tokens instead (#13)

Updated release workflow to use personal tokens instead

([07bc073](https://github.com/rijuma/ctxhelpr/commit/07bc073f41995adebdf7142e02628e110c63cd4f)) ([1a99949](https://github.com/rijuma/ctxhelpr/commit/1a99949816b3bd4d74bfcb759cace15856c5fbdd))
- Confirm new release v1.0.0 (#15)

## Release v1.0.0

Bumps version from 0.2.0 to 1.0.0 (major).

### Changelog

## [1.0.0] - 2026-02-12

### Bug Fixes

- Tag release fix
([573cf79](https://github.com/rijuma/ctxhelpr/commit/573cf792c5f5375e74f92d36c435995230edb9de)) ([e789a63](https://github.com/rijuma/ctxhelpr/commit/e789a63f344cb19ed126680e9897d87e40ff3975))
- Confirm new release v1.0.1 (#16)

## Release v1.0.1

Bumps version from 1.0.0 to 1.0.1 (patch).

### Changelog

## [1.0.1] - 2026-02-12

### Miscellaneous

- Fix tag and build
([c1efc91](https://github.com/rijuma/ctxhelpr/commit/c1efc9107951d60346c86982be445260198f03ef)) ([e5f02dd](https://github.com/rijuma/ctxhelpr/commit/e5f02ddd34a0519d625c03e918d6467e0ecbbb84))
- Confirm new release v1.0.2 (#17)

## Release v1.0.2

Bumps version from 1.0.1 to 1.0.2 (patch).

### Changelog

## [1.0.2] - 2026-02-12

### Bug Fixes

- Fixed x86 apple build
([0d3393a](https://github.com/rijuma/ctxhelpr/commit/0d3393adeaa4c090a161d861f098f06dd045ce35))

### Miscellaneous

- Improved asset name for builds
([18d19fc](https://github.com/rijuma/ctxhelpr/commit/18d19fc595182199fd30f92d76ca47472edf65e9)) ([216ae32](https://github.com/rijuma/ctxhelpr/commit/216ae32299bec170637948140b4aad21e46755d4))
- Added config options and updated incexing logic (#19)

# Summary

- TOML → JSON: Project config file is now .ctxhelpr.json instead of
.ctxhelpr.toml, aligning with the MCP ecosystem (JSON everywhere) and
removing the
Rust-centric TOML dependency
- Config CLI: New ctxhelpr config subcommand with init, validate, and
show actions so users can scaffold, check, and inspect their
configuration without
guessing
- No more silent failures: Config parse errors are now logged as
warnings instead of silently falling back to defaults. Unknown fields
are rejected
(deny_unknown_fields) to catch typos early

##  Why

Two problems with the old system:

1. .ctxhelpr.toml was a Rust-ism. ctxhelpr targets all languages and
lives in the MCP ecosystem where JSON is the standard config format
(settings.json,
mcp.json, etc.). TOML created unnecessary friction for non-Rust users.
2. ConfigCache::get() used unwrap_or_default(), meaning a user with a
typo in their config would silently get default behavior with zero
indication
anything was wrong. No way to validate a config file without reading
source code.

##  What changed

Config parser — Switched from toml::from_str to serde_json::from_str.
Introduced a ConfigError enum (NotFound / InvalidJson / IoError) so
callers get
structured errors with line/column info from serde_json. Added
Config::validate() for explicit validation (returns NotFound when no
file exists, unlike
load() which returns defaults). All config structs now have
#[serde(deny_unknown_fields)] and derive Serialize for JSON output.

Error handling — ConfigCache::get() now uses tracing::warn! on parse
failures instead of silently defaulting. Errors surface in MCP server
logs at warn
level.

CLI tooling — ctxhelpr config init scaffolds a .ctxhelpr.json with all
defaults filled in. ctxhelpr config validate [--path] checks file
existence, JSON
syntax, and schema validity, reporting specific errors. ctxhelpr config
show [--path] prints the resolved config (defaults merged with user
overrides).

Dependency cleanup — Removed the toml crate. serde_json was already a
dependency.

Documentation — Both READMEs (EN/ES) and docs/indexing-strategy.md
updated: TOML references → JSON, added config CLI docs, added field
reference table. ([0fbd7b5](https://github.com/rijuma/ctxhelpr/commit/0fbd7b55e60057c9caaf47e13f7bb09e2d3214e8))

