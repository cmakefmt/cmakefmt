# `cmakefmt`

**A blazing-fast, workflow-first CMake formatter — built in Rust, built to last.**

`cmakefmt` was born from frustration with `cmake-format`, the beloved-but-aging
Python tool from the `cmakelang` project. Instead of patching around its limits,
`cmakefmt` starts from scratch: a native Rust binary that respects your time,
your CI budget, and your build system.

Same spirit. No Python. No compromises.

## Why `cmakefmt`?

- **20× faster — not a typo.** `cmakefmt` hits a `20.77x` geometric-mean
  speedup over `cmake-format` on real-world CMake corpora. Pre-commit hooks that
  once made you wince now finish before you blink.
- **Zero dependencies. One binary.** No Python environment, no virtualenv
  bootstrap, no "works on my machine" dependency drift. Drop it in CI and forget
  about it.
- **Built for actual workflows.** `--check`, `--diff`, `--staged`, `--changed`,
  `--files-from`, `--show-config`, `--explain-config`, JSON reporting — the
  power-user features that `cmake-format` made you script around are all first-
  class citizens here.
- **Knows your commands.** Teach `cmakefmt` about your project's custom CMake
  functions and macros. No more generic token wrapping for functions *you* wrote.
- **Errors that actually help.** Parse and config failures come with file/line
  context, source snippets, and reproduction hints — not a wall of opaque parser
  noise.
- **Designed for real repositories.** Comment preservation, disabled regions,
  config discovery, ignore files, Git-aware file selection, and opt-in
  parallelism are core features, not afterthoughts.

## Performance Snapshot

The numbers speak for themselves:

| Metric | Signal |
| --- | --- |
| Geometric-mean speedup vs `cmake-format` | **`20.77x`** |
| End-to-end `format_source` on large synthetic input | `8.6263 ms .. 8.8934 ms` |
| Parser-only large synthetic input | `6.9304 ms .. 6.9677 ms` |
| Serial whole-corpus batch | `109.5 ms ± 1.3 ms` |
| `--parallel 8` whole-corpus batch | **`31.9 ms ± 1.0 ms`** |

Fast enough for local dev, pre-commit hooks, editor integrations, *and* CI — all
at once. The only question is: why settle for slower?

Curious about methodology? Evaluating whether it's worth switching?
Read [Performance](performance.md) and [Migration From `cmake-format`](migration.md).

## Quick Start

Install from this repository:

```bash
cargo install --path .
```

Dump a starter config:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

Check your entire repository without touching a single file:

```bash
cmakefmt --check .
```

Rewrite everything in place:

```bash
cmakefmt --in-place .
```

Format only the CMake files you're about to commit:

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
- Git-aware workflows: `--staged`, `--changed`, and `--since`
- rich debug output for discovery, config resolution, barriers, command forms, and layout choices
- opt-in parallel execution for multi-file runs
- a built-in command registry audited through CMake `4.3.1`

## Suggested Reading Order

New here?

1. [Install](install.md)
2. [CLI Reference](cli.md)
3. [Config Reference](config.md)
4. [Formatter Behavior](behavior.md)

Migrating from `cmake-format`?

1. [Migration From `cmake-format`](migration.md)
2. [Config Reference](config.md)
3. [Troubleshooting](troubleshooting.md)

Embedding `cmakefmt` as a library?

1. [Library API](api.md)
2. [Architecture](architecture.md)

## Common Workflows

Preview which files would change before touching anything:

```bash
cmakefmt --list-files .
```

See the exact patch instead of applying it:

```bash
cmakefmt --diff CMakeLists.txt
```

Trace which config file a target will actually use:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
```

Inspect the fully resolved, effective config:

```bash
cmakefmt --show-config src/CMakeLists.txt
```

## Current Status

`cmakefmt` is pre-`1.0` — honest about it, but already genuinely useful. Active
development is focused on:

- polishing documentation and the onboarding experience
- tightening release and distribution channels
- staying well ahead of `cmake-format` on performance and workflow ergonomics

Hit something unexpected? Start with [Troubleshooting](troubleshooting.md), then
reach for `--debug` and `--explain-config` before filing a bug report.
