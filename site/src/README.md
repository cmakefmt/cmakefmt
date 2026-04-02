# `cmakefmt` Documentation

`cmakefmt` is a Rust CMake formatter intended to replace `cmake-format` with a
single fast binary that is easy to automate in CI, pre-commit, and editor
workflows.

## What This Book Covers

- installation and local usage
- full CLI reference
- full config reference
- formatter behavior and known differences
- migration guidance for `cmake-format` users
- crate/API entry points
- high-level architecture notes
- changelog and release-note policy

## Quick Start

```bash
cargo install --path .
cmakefmt --check .
cmakefmt -i .
```

## Current Scope

The project currently supports:

- recursive discovery of CMake files
- in-place formatting and `--check`
- comment preservation
- config-file loading and per-command overrides
- barriers and fence passthrough
- debug mode
- opt-in parallel execution

The formatter is still under active development. Full module-command coverage,
very-large-codebase parallel surveying, and release/distribution work are still
in progress.
