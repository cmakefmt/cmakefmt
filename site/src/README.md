# `cmakefmt`

A very fast, workflow-first CMake formatter implemented in Rust.

`cmakefmt` is inspired by the original Python `cmake-format` from the
`cmakelang` project. The goal is not to dismiss that tool. It solved a real
problem for CMake users for years. `cmakefmt` tries to keep the parts people
liked there, then push much harder on speed, diagnostics, modern workflow
integration, and long-term maintainability.

## Why use `cmakefmt`?

- **Fast enough to use everywhere.** On the current local real-world corpus,
  `cmakefmt` measured a `20.77x` geometric-mean speedup over `cmake-format`.
- **Single native binary.** No Python environment, no virtualenv bootstrap,
  and no dependency drift in CI.
- **Workflow-first CLI.** `--check`, `--diff`, `--staged`, `--changed`,
  `--files-from`, `--show-config`, `--explain-config`, JSON reporting, and
  better config/debug tooling are built in.
- **Custom-command aware.** You can teach the formatter about project-specific
  CMake functions and macros instead of living with generic token wrapping.
- **Clearer failures.** Parse and config errors include file/line context,
  source snippets, and repro hints instead of opaque parser noise.
- **Designed for real repositories.** Comment preservation, disabled regions,
  config discovery, ignore files, Git-aware selection, and opt-in parallelism
  are all part of the core experience.

## Performance Snapshot

The benchmark environment and raw methodology are documented in the repository
performance notes, but the high-level signal is already strong:

| Metric | Current local signal |
| --- | --- |
| Geometric-mean speedup vs `cmake-format` | `20.77x` |
| End-to-end `format_source` on large synthetic input | `8.6263 ms .. 8.8934 ms` |
| Parser-only large synthetic input | `6.9304 ms .. 6.9677 ms` |
| Serial whole-corpus batch | `109.5 ms ± 1.3 ms` |
| `--parallel 8` whole-corpus batch | `31.9 ms ± 1.0 ms` |

Those numbers will change over time, but the pattern matters: `cmakefmt` is
already fast enough to be comfortable in local development, pre-commit hooks,
editor integrations, and CI.

If you are evaluating whether it is worth switching, read
[Performance](performance.md) and [Migration From `cmake-format`](migration.md)
next.

## Quick Start

Install from this repository:

```bash
cargo install --path .
```

Create a starter config:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

Check a repository without rewriting files:

```bash
cmakefmt --check .
```

Rewrite files in place:

```bash
cmakefmt --in-place .
```

Format only staged CMake files before you commit:

```bash
cmakefmt --staged --check
```

## What `cmakefmt` Covers Today

- recursive discovery of `CMakeLists.txt`, `*.cmake`, and `*.cmake.in`
- in-place formatting, stdout formatting, `--check`, `--diff`, and JSON reports
- YAML or TOML config files, with YAML preferred for larger user configs
- custom command specs and per-command formatting overrides
- comment preservation, fence/barrier passthrough, and markup-aware handling
- config introspection with `--show-config`, `--show-config-path`, and `--explain-config`
- Git-aware workflows such as `--staged`, `--changed`, and `--since`
- debug output for discovery, config resolution, barriers, command forms, and layout choices
- opt-in parallel execution for multi-file runs
- a built-in command registry audited through CMake `4.3.1`

## Suggested Reading Order

If you are new to the tool:

1. [Install](install.md)
2. [CLI Reference](cli.md)
3. [Config Reference](config.md)
4. [Formatter Behavior](behavior.md)

If you are migrating from `cmake-format`:

1. [Migration From `cmake-format`](migration.md)
2. [Config Reference](config.md)
3. [Troubleshooting](troubleshooting.md)

If you want to embed `cmakefmt` as a library:

1. [Library API](api.md)
2. [Architecture](architecture.md)

## Common Workflows

See which files would change:

```bash
cmakefmt --list-files .
```

Show the actual patch instead of rewriting the file:

```bash
cmakefmt --diff CMakeLists.txt
```

Inspect which config file a target will use:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
```

Inspect the fully resolved effective config:

```bash
cmakefmt --show-config src/CMakeLists.txt
```

## Current Status

`cmakefmt` is still pre-`1.0`, so some behavior can still evolve. The project
is already useful now, but alpha-release work is still focused on:

- polishing the documentation and onboarding experience
- tightening release/distribution channels
- keeping performance and workflow ergonomics ahead of `cmake-format`

If you hit an issue, start with [Troubleshooting](troubleshooting.md) and then
use `--debug` plus `--explain-config` before filing a bug report.
