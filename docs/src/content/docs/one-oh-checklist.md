---
title: 1.0 Checklist
description: Criteria that must be met before cutting cmakefmt 1.0.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

This page tracks the criteria for releasing cmakefmt 1.0. All items must
be checked before the tag is cut.

## Formatting quality

- [ ] Trailing-comment handling produces correct, stable output for all
      common patterns
- [ ] `set()` formatting reviewed and agreed for all usage patterns
      (simple, list, cached, PARENT_SCOPE, env)
- [ ] `enable_sort` and `autosort` shipped and tested on real-world
      corpora
- [ ] No known formatting regressions in the real-world corpus suite

## API stability

- [x] Parser internals (`Rule` enum) hidden from public API
- [x] Error types are crate-owned (`ParseError`, `ConfigError`, `SpecError`)
- [x] `CommandConfig` does not expose internal representation fields
- [x] `Experimental` struct is `#[non_exhaustive]`
- [ ] `cargo doc --no-deps` produces no warnings
- [ ] All public types and functions have rustdoc comments

## Config schema

- [x] `[experimental]` section exists for gating unstable options
- [ ] JSON schema registered with SchemaStore is current
- [ ] `cmakefmt config dump` output round-trips through
      `cmakefmt config check` without errors

## CLI

- [x] `--preview` flag enables all experimental options
- [x] `--list-unknown-commands` shipped
- [x] `--watch` and `--explain` shipped
- [x] `--progress-bar` works with all applicable modes
- [x] `.editorconfig` fallback with `--no-editorconfig`
- [ ] All deprecated flags from the subcommand migration are removed or
      have a clear deprecation timeline

## Documentation

- [x] Stability contract published at `/stability/`
- [x] Migration guide at `/migration/`
- [x] Editor integration for VS Code, Neovim, Helix, Zed, Emacs
- [x] CI integration for GitHub Actions, GitLab CI, Azure Pipelines,
      Bitbucket Pipelines
- [ ] Versioned docs with version selector
- [ ] This checklist is fully checked

## Testing

- [x] Format-on-save verified in VS Code
- [ ] Format-on-save verified in Neovim
- [ ] Large-file LSP timeout test (>= 1000 lines)
- [ ] All real-world corpus fixtures pass idempotency
- [ ] Cross-platform output consistency (Linux, macOS, Windows)

## Distribution

- [x] GitHub Releases with signed binaries
- [x] crates.io
- [x] Homebrew tap
- [x] PyPI (binary-only wheels)
- [x] conda-forge
- [x] winget
- [x] VS Code Marketplace + Open VSX
- [x] Docker image on GHCR
- [x] GitHub Action

## Benchmarks

- [ ] Baseline stored and tracked per release
- [ ] Regression review policy documented
- [ ] Scheduled CI benchmark job running
