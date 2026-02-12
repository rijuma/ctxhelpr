# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-02-12

### Miscellaneous

- First commit ([4709814](https://github.com/rijuma/ctxhelpr/commit/4709814c96e41d3a1b477ba1596aa061969bc5a5))

### Other

- Add workflows (#1)

Added github workflows for CI

Co-authored-by: Rijuma <marcos@rigoli.dev> ([d3b9289](https://github.com/rijuma/ctxhelpr/commit/d3b928998c73ba3a1a3b57a3c448538f7b1d0f7f))
- Add local and global scopes and add default permission handling (#2)

- Added possibility to manage two setting scopes, local and global:
- The local scope allows to enable indexing, skill and permissions
locally.
- The global scope enables the indexing and the skill to Claude
globally.
- Added prompting to handle new options and flags to skip the prompting.

Co-authored-by: Rijuma <marcos@rigoli.dev> ([98275ed](https://github.com/rijuma/ctxhelpr/commit/98275edcc7789be06cdf76117929cc2e33d58f52))
- Rename setup for install and added release workflows (#3)

- Renamed the `setup` command to `install`, to be more clear and aligned
with the opposite `uninstall` we are already using.
- Added changelog logic, tag actions and release build workflows.
- Updated docs.

Co-authored-by: Rijuma <=> ([498c2c5](https://github.com/rijuma/ctxhelpr/commit/498c2c540c956b6dbacc999a3ad97f705c9e09e8))
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

Co-authored-by: Rijuma <=> ([ff45a2f](https://github.com/rijuma/ctxhelpr/commit/ff45a2fb394064c3148f79d844356dcbeecc6f0a))
- Improve release workflow (#5)

- Split release workflow to be more efficient.

Co-authored-by: Rijuma <=> ([c14bd23](https://github.com/rijuma/ctxhelpr/commit/c14bd23262289989c9dc6b192e69c8ffcd725ad0))
- Fixed release workflow (#7)

- Updated PR content and fixed CI workflows never starting.

Co-authored-by: Rijuma <=> ([b93fed1](https://github.com/rijuma/ctxhelpr/commit/b93fed1993a2b04adcc3ae9b963d0e20a0ec5f8c))
- Fix release workflow (#11)

## Summary
- Use REST API for PR creation to avoid `GITHUB_TOKEN` GraphQL
permission issue ("Resource not accessible by integration")
- Add `workflow_dispatch` trigger to CI so release workflow can trigger
it
- Add resilience for re-runs: delete pre-existing remote branch before
push
- Improve PR description with actual changelog content

Co-authored-by: Rijuma <=> ([3233097](https://github.com/rijuma/ctxhelpr/commit/3233097f2d92a89dc077e3dd62ab58f2dc5591b0))
- Updated release workflow to use personal tokens instead (#13)

Updated release workflow to use personal tokens instead

Co-authored-by: Rijuma <=> ([cbdd6fc](https://github.com/rijuma/ctxhelpr/commit/cbdd6fc95a20c056bc2ac6948a70258c9b00fdbc))

