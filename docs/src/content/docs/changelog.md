---
title: Changelog
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

This project follows a simple changelog discipline:

- keep user-visible changes in `Unreleased` until the next cut
- group entries by impact, not by file
- call out migration-impacting changes explicitly

## Unreleased

### Removed

- `--lsp` flag — use `cmakefmt lsp` instead
- `--generate-completion` flag — use `cmakefmt completions <SHELL>` instead
- `--dump-config` flag — use `cmakefmt config dump` instead
- `--dump-schema` flag — use `cmakefmt config schema` instead
- `--check-config` flag — use `cmakefmt config check` instead
- `--show-config` flag — use `cmakefmt config show` instead
- `--show-config-path` / `--find-config-path` flag — use `cmakefmt config path`
  instead
- `--explain-config` flag — use `cmakefmt config explain` instead
- `--convert-legacy-config` / `--convert-legacy-config-format` flags — use
  `cmakefmt config convert` instead
- `cmakefmt init` (top-level subcommand) — use `cmakefmt config init` instead

### Added

- `cmakefmt config show`, `cmakefmt config path`, and `cmakefmt config explain`
  now accept an optional file path argument directly (e.g.
  `cmakefmt config show src/CMakeLists.txt`)
- File-not-found validation for `config show`, `config path`, and
  `config explain` — clear error message when the target file does not exist

### Changed

- `cmakefmt config dump` and `cmakefmt config show` format is now specified
  via `--format` flag (e.g. `cmakefmt config dump --format toml`) instead of
  a positional argument
- 78 `conflicts_with` annotations removed from the CLI definition — subcommand
  structure now handles mutual exclusivity

## 0.5.0 — 2026-04-10

## 0.5.0 — 2026-04-10

### Added

- `cmakefmt config` subcommand group — `dump`, `schema`, `check`, `show`,
  `path`, `explain`, `convert`, and `init` sub-subcommands for config
  inspection and conversion
- `cmakefmt lsp` subcommand (replaces `--lsp` flag, which is now deprecated)
- `cmakefmt completions <SHELL>` subcommand (replaces `--generate-completion`
  flag, which is now deprecated)
- `cmakefmt install-hook` subcommand — one-command git pre-commit hook setup
- LSP: `workspace/didChangeConfiguration` support — live config reload when
  `.cmakefmt.yaml` changes without restarting the server
- LSP: `textDocument/codeAction` — "Disable cmakefmt for selection" action
  that inserts `# cmakefmt: off/on` barriers
- LSP: 10-second timeout on formatting requests to prevent pathological inputs
  from freezing the editor
- Colored CLI help output (green headers, cyan flags)
- Cargo-fuzz targets for parser and formatter (`fuzz/`)
- `cmakefmt-fix` pre-commit hook for auto-formatting (in addition to the
  existing check-only `cmakefmt` hook)
- Docker image published to GHCR (`ghcr.io/cmakefmt/cmakefmt`) on every
  release
- Cross-platform output consistency CI workflow
- Real-world regression suite CI workflow (CMake, LLVM, OpenCV)
- SBOM generation (`cargo-cyclonedx`) in the release workflow
- Docker build CI workflow
- WASM API documentation page
- Docs site redesign: Plus Jakarta Sans headings, gradient hero with animated
  dot grid, animated link underlines, card lift-on-hover, sidebar active
  indicator, page load fade-in
- Benchmarks for config pattern validation, legacy conversion, and atomic
  writes

### Changed

- Unrecognized config keys now produce an error instead of being silently
  ignored (`deny_unknown_fields` on all config sections)
- `require_valid_layout` error now suggests the specific `line_width` value
  needed to accommodate the offending line
- Config regex patterns (`literal_comment_pattern`, `explicit_trailing_pattern`,
  etc.) are now compiled once per formatting run instead of once per comment
  or command, improving performance on comment-heavy files

### Deprecated

- `--lsp` flag — use `cmakefmt lsp` instead
- `--generate-completion` flag — use `cmakefmt completions <SHELL>` instead
- `--dump-config` flag — use `cmakefmt config dump` instead
- `--dump-schema` flag — use `cmakefmt config schema` instead
- `--check-config` flag — use `cmakefmt config check` instead
- `--show-config` flag — use `cmakefmt config show` instead
- `--show-config-path` flag — use `cmakefmt config path` instead
- `--explain-config` flag — use `cmakefmt config explain` instead
- `--convert-legacy-config` flag — use `cmakefmt config convert` instead
- `cmakefmt init` (top-level) — use `cmakefmt config init` instead

## 0.4.0 — 2026-04-09

### Added

- `cmakefmt init` subcommand — generates a starter `.cmakefmt.yaml` in the
  current directory
- `--check-config` flag — validates a config file and exits without formatting
- `--stat` flag — prints a git-style summary (`3 files changed, 12 lines
  reformatted`)
- Elapsed time shown in the formatting summary (e.g. `in 0.42s`)
- Fix hint printed when `--check` fails (`hint: run cmakefmt --in-place .`)
- User-friendly panic handler with structured bug report template
- `.pre-commit-hooks.yaml` for pre-commit integration
- `Dockerfile` for CI usage
- GitHub issue templates for bug reports and feature requests
- Architecture guide for contributors
- FAQ docs page
- LSP-mode editor configs for Neovim, Helix, and Zed
- Azure Pipelines and Bitbucket Pipelines examples in CI docs
- Migration guide expanded with key differences and unsupported options tables

### Fixed

- `--diff` now works with `--check` and non-human `--report-format` modes;
  previously both suppressed the unified diff output

### Security

- Config regex patterns (`literal_comment_pattern`, `explicit_trailing_pattern`,
  `fence_pattern`, `ruler_pattern`) are now validated at config load time;
  previously invalid or pathological regexes were silently accepted and could
  cause CPU exhaustion (ReDoS)
- `--in-place` writes are now atomic (write to temp file, then rename);
  previously a TOCTOU race could cause unintended overwrites if the target
  file was replaced with a symlink between read and write

## 0.3.0 — 2026-04-08

### Added

- `--dump-schema` flag — prints the JSON Schema for the `.cmakefmt.yaml` /
  `.cmakefmt.toml` config file to stdout and exits; schema is also published
  at `cmakefmt.dev/schemas/latest/schema.json` for zero-config YAML
  autocomplete in editors with `redhat.vscode-yaml` or similar plugins
- `--lsp` flag — starts a stdio JSON-RPC Language Server Protocol server
  supporting `textDocument/formatting` and `textDocument/rangeFormatting`;
  enables format-on-save in any editor with LSP client support (Neovim,
  Helix, Zed, Emacs, …) without a dedicated extension
- Guide pages on [cmakefmt.dev](https://cmakefmt.dev): editor integration,
  CI integration, tool comparison, badge, and "Projects using cmakefmt"

## 0.2.0 — 2026-04-07

### Added

- interactive browser playground at [cmakefmt.dev/playground](https://cmakefmt.dev/playground/) —
  format CMake code, edit config, and define custom command specs entirely in
  the browser via WebAssembly
- `format.disable` config option — global kill-switch that returns the source
  file unchanged; useful for temporarily opting out of formatting without
  removing the config file
- `format.line_ending` config option — controls output line endings: `unix`
  (LF, default), `windows` (CRLF), or `auto` (detects predominant ending in
  the input and preserves it)
- `format.always_wrap` config option — list of command names that are always
  rendered in vertical (wrapped) layout, never inline or hanging; the
  `always_wrap` flag in per-command specs (`commands:`) is now also honoured
- `format.require_valid_layout` config option — when `true`, the formatter
  returns an error if any output line exceeds `line_width`; useful for strict
  CI enforcement
- `format.fractional_tab_policy` config option — controls sub-tab-stop
  indentation remainders when `use_tabs` is `true`: `use-space` (default)
  keeps them as spaces, `round-up` promotes them to a full tab
- `format.max_rows_cmdline` config option — maximum number of rows a
  positional argument group may occupy before the hanging-wrap layout is
  rejected and vertical layout is used instead (default: `2`)
- `markup.explicit_trailing_pattern` config option — regex pattern (default
  `#<`) that marks an inline comment as trailing its preceding argument,
  keeping it on the same line rather than wrapping to a new line

## 0.1.1 — 2026-04-06

### Added

- Homebrew installation support (`brew install cmakefmt/cmakefmt/cmakefmt`)
- shell completion installation instructions
- site metadata and crate status badge on [docs.rs](https://docs.rs)

### Changed

- improved [docs.rs](https://docs.rs) readability and tightened public API surface
- documentation clarity and wording improvements

## 0.1.0 — 2026-04-05

### Added

- full CLI workflow: `--check`, `--diff`, `--in-place`, `--staged`,
  `--changed`, `--files-from`, `--parallel`, `--dump-config`, `--list-input`,
  `--list-changed`, `--explain-config`, `--quiet`, `--keep-going`
- recursive file discovery with `.cmakefmtignore` and `--exclude-regex` support
- YAML and TOML config file support with automatic discovery
- comment preservation and fence/barrier support (`# cmakefmt: off/on`)
- pragma-gated rollout mode
- formatter result caching
- colored diff output and in-place progress bar
- CI-oriented report formats (JSON, JUnit, SARIF, GitHub Actions, GitLab CI)
- legacy `cmake-format` config conversion (`--convert-config`)
- built-in and module-command spec coverage audited against CMake 4.3.1
- custom command specifications via config
- real-world regression corpus covering LLVM, Qt, protobuf, and more
- performance benchmarks: ~20× geometric-mean speedup over `cmake-format`
- parallel formatting with `--parallel`
- comprehensive docs site at [cmakefmt.dev](https://cmakefmt.dev)
- shell completion generation (`--completions`)
- dual MIT/Apache-2.0 licensing with full REUSE compliance
- Windows, macOS, and Linux support

### Compatibility Notes

- `cmakefmt` aims to be easy to migrate to from `cmake-format`, but output is
  not intended to be byte-for-byte identical
- config option names differ from `cmake-format` in places; use
  `--convert-config` to migrate
