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

| Feature | `cmakefmt` | cmake-format | gersemi |
|---|---|---|---|
| Language | Rust | Python | Python |
| Install | Homebrew, cargo, binary | pip | pip |
| Config file | YAML or TOML | Python, YAML, or JSON | None |
| Config discovery | Walks up the directory tree | Directory-based | None |
| Migrate existing config | `cmakefmt config convert` | — | — |
| Check mode | `--check` | `--check-only` | `--check` |
| Diff output | `--diff` | — | — |
| Stdin formatting | `--stdin-path <path> -` | via `--` | via stdin |
| Range formatting | `--lines` | — | — |
| GitHub Action | `cmakefmt/cmakefmt-action@v1` | — | — |
| CI annotations | `--report-format github` | — | — |
| VS Code extension | First-party | First-party | — |
| Active maintenance | Yes | Minimal | Yes |
| License | MIT OR Apache-2.0 | Apache-2.0 | MIT |

## Performance

`cmakefmt` is a native binary with no interpreter startup overhead. On a
cold invocation it formats a typical `CMakeLists.txt` in under 5 ms. Python
tools (`cmake-format`, `gersemi`) typically spend 200–500 ms just starting the
interpreter before any file is touched — noticeable on every save in an editor.

For large repositories `cmakefmt` also ships a content-addressed cache
(`--cache`) so unchanged files are not re-parsed between runs.

## Configuration

`cmakefmt` supports YAML and TOML config files (`.cmakefmt.yaml`,
`.cmakefmt.toml`) with a structured, documented schema. Config is discovered
automatically by walking up from the file being formatted to the filesystem
root, mirroring how tools like `rustfmt` and `ruff` work.

`cmake-format` supports a wider variety of config formats (Python files,
YAML, JSON) but discovery is less structured. `gersemi` has no config file at
all — its formatting is entirely opinionated.

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
installs the right binary for the runner OS in one `uses:` line. It also
outputs GitHub Actions annotations via `--report-format github`, Checkstyle
XML, JUnit XML, and SARIF JSON for downstream tooling.

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
