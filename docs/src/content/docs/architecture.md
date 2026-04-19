---
title: Architecture
description: How `cmakefmt` works — a guide for contributors.
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

Six modules drive the pipeline and its supporting infrastructure:

| Module | Path | Purpose |
|---|---|---|
| Parser | `src/parser/` | CMake source to AST |
| Config | `src/config/` | Formatting options, config-file loading, and legacy conversion |
| Spec Registry | `src/spec/` | Command argument structure definitions |
| Formatter | `src/formatter/` | AST + config + specs to formatted text |
| Errors | `src/error.rs` | Crate-owned error and diagnostic types |
| File Discovery | `src/files.rs` | Recursive CMake file discovery and ignore handling |

Additional compile-time targets:

| Module | Path | Feature gate | Purpose |
|---|---|---|---|
| LSP Server | `src/lsp/` | `lsp` | JSON-RPC formatting server for editors |
| WASM | `src/wasm.rs` | `wasm32` target | Browser playground entry points |

Each module has a clean boundary. The parser knows nothing about formatting, the
spec registry knows nothing about config, and the formatter consumes all three.

## Parser (`src/parser/`)

The parser is a hand-written recursive-descent implementation over a streaming
scanner. It turns CMake source text into an AST with this structure:

```text
File -> Statement* -> CommandInvocation -> Argument*
```

`Argument` nodes cover unquoted args, quoted strings, bracket arguments, and
inline comments. Comments are preserved as `InlineComment` arguments so they
survive round-tripping through the formatter.

The public entry point is `parser::parse()` in `src/parser/mod.rs`, which
coordinates four private layers:

- `cursor.rs` for byte-level traversal
- `scanner.rs` for parser-driven literal/comment/argument scanning
- `grammar.rs` for structural parsing into a private parse tree
- `lower.rs` for blank-line and trailing-comment normalization into the public AST

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
output.

### Entry points

The public API in `mod.rs` exposes several entry points:

- **`format_source()`** -- format raw source with the built-in registry.
- **`format_source_with_registry()`** -- format raw source with a custom
  registry (built-ins merged with user overrides).
- **`format_parsed_file()`** -- format an already-parsed AST, avoiding
  re-parsing when the same file is formatted with different configs.
- **`format_source_with_debug()`** / **`format_source_with_registry_debug()`**
  -- variants that also return debug lines describing formatting decisions.

All entry points validate the runtime config via `validate_runtime_config()`
before formatting.

### Barrier handling

`format_source_impl()` in `mod.rs` handles the full file: it walks the source
line-by-line, detecting barrier regions (`# cmakefmt: off/on`, `# fmt: off/on`,
`# cmake-format: off/on`, and fence barriers `# ~~~`). Lines inside a barrier
region are emitted verbatim. Lines outside barrier regions are collected and
parsed as chunks, then formatted.

### Command formatting

`format_command()` in `node.rs` handles individual command invocations. It
splits arguments into sections using the command's `CommandForm`, then tries
three layouts in order:

1. **Inline** -- everything on a single line.
2. **Hanging** -- continuation lines aligned to the opening parenthesis.
3. **Vertical** -- one argument (or keyword group) per line.

The first layout that fits within `line_width` wins. This gives stable,
predictable wrapping without ad-hoc heuristics.

### Comment handling

`comment.rs` provides comment formatting helpers, including comment reflowing
and alignment.

## Config (`src/config/`)

The `Config` struct holds all formatting options: `line_width`, `tab_size`,
`command_case`, `keyword_case`, `dangle_parens`, and more. It is loaded from
`.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` with automatic
discovery up the directory tree (and a home-directory fallback).

Per-command overrides allow different settings for specific commands via the
`per_command:` config section, resolved at format time by
`config.for_command()`.

Key files:

- **`mod.rs`** -- `Config` struct, `CaseStyle`, `LineEnding`, and related types.
- **`file.rs`** -- config file discovery, deserialization (`FileConfig` schema),
  merge logic, `from_yaml_str()` for validated parsing, and the default config
  template.
- **`legacy.rs`** -- legacy cmake-format config conversion (`cmakefmt config convert`).

The `FileConfig` struct in `file.rs` defines the on-disk schema with `format:`
and `markup:` sections. `Config::from_yaml_str()` parses through `FileConfig`
so that invalid fields are rejected rather than silently ignored.

## Errors (`src/error.rs`)

Crate-owned error types used across parsing, config loading, and formatting:

- **`Error`** -- the top-level error enum with variants for config errors, parse
  errors (with source context), I/O errors, and formatting failures.
- **`ParseDiagnostic`** -- crate-owned parser diagnostic with line, column, and
  a human-readable message. The parser tracks byte offsets internally and
  resolves line/column only when constructing this public error.
- **`FileParseError`** -- structured metadata for config/spec deserialization
  failures (format name, message, optional line/column).

## File Discovery (`src/files.rs`)

Recursive CMake file discovery for the CLI, built on the `ignore` crate (the
same walking engine used by ripgrep). Handles:

- `.cmakefmtignore` custom ignore files
- Git ignore file honoring (optional)
- Regex-based file filtering
- Sorted output for deterministic CLI ordering

Only compiled when the `cli` feature is enabled; not part of the library
embedding API.

## LSP Server (`src/lsp/`)

The LSP server is a thin wrapper around `format_source()` that speaks JSON-RPC
on stdio. It is compiled only when the `lsp` feature is enabled.

It handles:

- `textDocument/formatting` -- whole-file formatting
- `textDocument/rangeFormatting` -- range formatting (backed by `--lines`)
- `textDocument/codeAction` -- "Disable `cmakefmt` for selection" action
- `workspace/didChangeConfiguration` -- live config reload

The entry point is `lsp::run()`, which uses the `lsp-server` crate for the
connection lifecycle. A 10-second timeout protects against pathological inputs.

## WASM (`src/wasm.rs`)

Entry points for the browser playground, compiled only for `wasm32` targets
via `wasm-bindgen`:

- **`format()`** -- format source with a YAML config string (same schema as
  `.cmakefmt.yaml`).
- **`default_config_yaml()`** -- return the default config as a YAML string.

Config is validated through `Config::from_yaml_str()` so the playground rejects
invalid fields the same way the CLI does.

## Where to Start

| Task | Start here |
|---|---|
| Change formatting behavior | `src/formatter/node.rs` |
| Add a new config option | `src/config/mod.rs` |
| Change the parser | `src/parser/{scanner,grammar,lower}.rs` |
| Add or update a built-in command spec | `src/spec/builtins.toml` |
| Add a new CLI flag | `src/main.rs` |
| Modify LSP behavior | `src/lsp/mod.rs` |
| Add a new error variant | `src/error.rs` |
| Change file discovery or ignore logic | `src/files.rs` |

See also [CONTRIBUTING.md](/contributing/) for the full checklist of files to
update when making changes.
