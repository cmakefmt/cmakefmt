---
title: Comparison
description: How `cmakefmt` compares to cmake-format and gersemi.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

There are three CMake formatters in active use: `cmakefmt`, `cmake-format`, and
`gersemi`. This page summarises the key differences to help you choose.

## At a Glance

| Feature | `cmakefmt` | `cmake-format` | `gersemi` |
|---|---|---|---|
| Command specs | 150+ (CMake 4.3) | ~100 (CMake 3.20) | Built-in |
| Active maintenance | Yes | No (last release 2021) | Yes |
| Recursive discovery | By default | No (explicit file list) | By default |
| Parallel formatting | By default | No | No |
| Language | Rust | Python | Python |
| Install | Homebrew, cargo, pip, conda, binary | pip | pip |
| LSP server | Built-in (`cmakefmt lsp`) | No | No |
| VS Code extension | First-party | First-party | No |
| Watch mode | `--watch` | No | No |
| Check mode | `--check` | `--check-only` | `--check` |
| Diff output | `--diff` | No | No |
| Git-aware selection | `--staged`, `--changed` | No | No |
| Range formatting | `--lines` | No | No |
| GitHub Action | `cmakefmt/cmakefmt-action` | No | No |
| CI report formats | GitHub, Checkstyle, JUnit, SARIF, JSON | No | No |
| Config file | YAML or TOML | Python, YAML, or JSON | None |
| Config discovery | Per-file, walks up to `.git` root | Directory-based | None |
| Config autocomplete | JSON Schema on SchemaStore | No | No |
| Migrate existing config | `cmakefmt config convert` | — | — |
| Parse tree dump | `cmakefmt dump ast/parse` | `--dump parse` | No |
| Stdin formatting | `--stdin-path <path> -` | via `--` | via stdin |
| License | MIT OR Apache-2.0 | Apache-2.0 | MIT |

## File Discovery and Parallelism

`cmakefmt` recursively discovers all `CMakeLists.txt` and `*.cmake` files
by default — just point it at your project root. It also formats them in
parallel using all available CPUs. On a 612-file repository this means
sub-second formatting of the entire project.

`cmake-format` requires you to pass every file explicitly (or write a
`find` + `xargs` wrapper). It has no parallel mode.

`gersemi` supports recursive discovery but has no parallel mode.

## Performance

`cmakefmt` is a native binary with no interpreter startup overhead. On a
cold invocation it formats a typical `CMakeLists.txt` in under 5 ms. Python
tools (`cmake-format`, `gersemi`) typically spend 200–500 ms just starting the
interpreter before any file is touched — noticeable on every save in an editor.

For large repositories `cmakefmt` also ships a content-addressed cache
(`--cache`) so unchanged files are not re-parsed between runs.

See [Performance](/performance/) for full benchmarks.

## Configuration

`cmakefmt` supports YAML and TOML config files (`.cmakefmt.yaml`,
`.cmakefmt.toml`) with a structured, documented schema. Config is discovered
automatically by walking up from the file being formatted to the filesystem
root, mirroring how tools like `rustfmt` and `ruff` work.

`cmakefmt` also publishes a JSON Schema to
[SchemaStore](https://www.schemastore.org/), which means editors with YAML
language support (VS Code with the Red Hat YAML extension, JetBrains IDEs,
Neovim with `yaml-language-server`) automatically provide autocomplete,
validation, and inline documentation for `.cmakefmt.yaml` — no per-user
setup required.

`cmake-format` supports a wider variety of config formats (Python files,
YAML, JSON) but has no schema and no autocomplete support. `gersemi` has
no config file at all — its formatting is entirely opinionated.

## Editor Integration

`cmakefmt` ships a first-party VS Code extension
([cmakefmt.vscode-cmakefmt](https://marketplace.visualstudio.com/items?itemName=cmakefmt.vscode-cmakefmt))
with format-on-save support. It works in Neovim via `conform.nvim`, Helix,
Zed, and Emacs via `apheleia` — see the [Editor Integration](/editors/) page.

`cmake-format` has a widely-used VS Code extension with over 500k installs.
`gersemi` has no editor integration.

## CI and Automation

`cmakefmt` ships a first-party GitHub Action
([cmakefmt/cmakefmt-action](https://github.com/cmakefmt/cmakefmt-action)) that
installs the right binary for the runner OS in one `uses:` line — see the [CI Integration](/ci/)
page. It also outputs GitHub Actions annotations via `--report-format github`,
Checkstyle XML, JUnit XML, and SARIF JSON for downstream tooling.

Neither `cmake-format` nor `gersemi` ship a GitHub Action.

## Maintenance Status

`cmake-format` last released in 2021 and is in maintenance-only mode. The
Python 3 ecosystem has moved on and there are known compatibility issues with
newer Python versions. `gersemi` is actively maintained. `cmakefmt` is under
active development with frequent releases.

## Migrating from cmake-format

`cmakefmt` can convert your existing `cmake-format` config automatically:

```bash
cmakefmt config convert .cmake-format.yaml > .cmakefmt.yaml
```

See the [Migration Guide](/migration/) for a full walkthrough.
