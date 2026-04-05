<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Releasing `cmakefmt`

This document is the release checklist and policy reference for the first
public alpha and the releases that follow it.

## Alpha Contract

`1.0.0-alpha.1` should mean:

- the formatter is ready for real repositories
- Linux, macOS, and Windows are supported
- the CLI, config, and diagnostics are intentionally designed
- output may still change between alpha releases when formatting bugs are fixed

It does **not** mean formatting stability is frozen. Teams that care about
churn should pin an explicit prerelease version.

## Release Checklist

1. Ensure the working tree is clean.
2. Run the standard validation suite:
   - `cargo fmt --all --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `bash scripts/check-docs.sh`
   - `reuse --no-multiprocessing lint`
3. Review performance and coverage status.
4. Review changelog and release notes.
5. Bump the package version.
6. Regenerate any release assets:
   - shell completions via `--generate-completion`
   - man page via `--generate-man-page`
7. Tag the release with `v<version>`.
8. Let the release workflow build artifacts and publish the release.
9. Smoke-test at least one release artifact manually.
10. Announce the release and link the changelog entry.

## Rollback / Yank

If a bad prerelease ships:

- yank the crates.io release
- mark the GitHub Release as superseded or broken
- document the issue in the changelog/release notes
- cut a fixed prerelease rather than rewriting the bad tag

## Channel Ownership

These are the intended support levels during alpha:

- Officially maintained:
  - GitHub Releases binaries
  - crates.io source package
  - docs site
  - Homebrew / `winget` / Scoop once added
- Best effort during alpha:
  - other package-manager wrappers
  - containers and ecosystem-specific integrations

## License / Packaging Expectations

- project license expression: `MIT OR Apache-2.0`
- `reuse lint` must stay green
- the published crate should remain curated and minimal
- release artifacts should include the relevant license material
