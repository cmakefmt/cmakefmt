---
title: Architecture
description: How cmakefmt works — a guide for contributors.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

## Overview

cmakefmt is a structured formatting pipeline:

```text
source text -> parser -> AST -> formatter -> formatted output
```

Four independent modules drive this pipeline:

| Module | Path | Purpose |
|---|---|---|
| Parser | `src/parser/` | CMake source to AST |
| Config | `src/config/` | Formatting options and config-file loading |
| Spec Registry | `src/spec/` | Command argument structure definitions |
| Formatter | `src/formatter/` | AST + config + specs to formatted text |

Each module has a clean boundary. The parser knows nothing about formatting, the
spec registry knows nothing about config, and the formatter consumes all three.

## Parser (`src/parser/`)

The parser uses [pest](https://pest.rs/) with a PEG grammar defined in
`src/parser/cmake.pest`. It turns CMake source text into an AST with this
structure:

```text
File -> Statement* -> CommandInvocation -> Argument*
```

`Argument` nodes cover unquoted args, quoted strings, bracket arguments, and
inline comments. Comments are preserved as `InlineComment` arguments so they
survive round-tripping through the formatter.

The entry point is `parser::parse()` in `src/parser/mod.rs`, which returns a
`File` AST node.

## Command Spec Registry (`src/spec/`)

The registry defines how each CMake command's arguments are structured:
positional arguments, keyword groups, and flags. This is what lets the formatter
distinguish semantic keywords like `PUBLIC` and `PRIVATE` from ordinary
arguments.

Built-in specs live in `src/spec/builtins.toml`. Each command maps to a
`CommandSpec`, which is either a single `CommandForm` or a discriminated set of
forms keyed by the first argument (e.g. `install(TARGETS ...)` vs
`install(FILES ...)`).

A `CommandForm` describes:

- positional argument slots with `NArgs` counts
- keyword groups and their expected argument counts
- flags (zero-argument keywords)

Users can override or extend the built-in registry via the `commands:` section
in their config file. The registry resolves each command invocation to the
appropriate `CommandForm`, which then guides formatting decisions.

## Formatter (`src/formatter/`)

The formatter takes the AST, config, and command registry and produces formatted
output. The key files are:

- **`mod.rs`** -- `format_source_impl` handles the full file: it walks
  statements, manages indentation for block commands (`if`/`endif`, etc.), and
  handles barrier regions.
- **`node.rs`** -- `format_command` handles individual command invocations. It
  splits arguments into sections using the command's `CommandForm`, then tries
  three layouts in order:

  1. **Inline** -- everything on a single line.
  2. **Hanging** -- continuation lines aligned to the opening parenthesis.
  3. **Vertical** -- one argument (or keyword group) per line.

  The first layout that fits within `line_width` wins. This gives stable,
  predictable wrapping without ad-hoc heuristics.

- **`comment.rs`** -- comment formatting helpers.

## Config (`src/config/`)

The `Config` struct holds all formatting options: `line_width`, `tab_size`,
`command_case`, `dangle_parens`, and more. It is loaded from
`.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` with automatic
discovery up the directory tree (and a home-directory fallback).

Per-command overrides allow different settings for specific commands via the
`per_command:` config section, resolved at format time by
`config.for_command()`.

Legacy cmake-format config conversion lives in `src/config/legacy.rs`.

Key files:

- **`mod.rs`** -- `Config` struct, `CaseStyle`, `LineEnding`, and related types.
- **`file.rs`** -- config file discovery, deserialization, merge logic, and the
  default config template.
- **`legacy.rs`** -- legacy cmake-format config conversion (`cmakefmt config convert`).

## LSP Server (`src/lsp/`)

The LSP server is a thin wrapper around `format_source()` that speaks JSON-RPC
on stdio. It is compiled only when the `lsp` feature is enabled.

It handles `textDocument/formatting` and `textDocument/rangeFormatting` requests.
The entry point is `lsp::run()`, which uses the `lsp-server` crate for the
connection lifecycle.

## Barrier Regions

`# cmakefmt: off` and `# cmakefmt: on` comments create regions that are passed
through verbatim. This is handled in `format_source_impl` (in
`src/formatter/mod.rs`) before the formatter processes individual commands --
statements inside a barrier region are emitted as-is.

## Where to Start

| Task | Start here |
|---|---|
| Change formatting behavior | `src/formatter/node.rs` |
| Add a new config option | `src/config/mod.rs` |
| Change the parser or grammar | `src/parser/cmake.pest` |
| Add or update a built-in command spec | `src/spec/builtins.toml` |
| Add a new CLI flag | `src/main.rs` |
| Modify LSP behavior | `src/lsp/mod.rs` |

See also [CONTRIBUTING.md](/contributing/) for the full checklist of files to
update when making changes.
