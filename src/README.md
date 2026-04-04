<!--
SPDX-FileCopyrightText: 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `src/`

This directory contains the implementation of `cmakefmt`.

## Layout

- `main.rs`
  - CLI entry point, argument parsing, target collection, and command dispatch
- `lib.rs`
  - public crate exports
- `error.rs`
  - shared error model
- `files.rs`
  - recursive CMake file discovery and filename filtering
- `config/`
  - runtime config model, config-file loading, default config template
- `parser/`
  - Pest grammar, AST conversion, and parser tests
- `formatter/`
  - AST-to-text formatting logic, comments, barriers, wrapping, and layout
- `spec/`
  - built-in/module command registry and mergeable user overrides

## Change Guidelines

- parser changes usually affect fixtures and snapshots
- formatter changes usually affect snapshots, idempotency, and real-world corpus output
- config changes usually affect CLI docs, dump-config output, and config tests
- spec changes should update `builtins.toml` and registry tests together
