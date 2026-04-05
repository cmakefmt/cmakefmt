<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Architecture

A user-facing overview of how `cmakefmt` works and why it is built the way it
is.

## Mental Model

`cmakefmt` is not a regex-based text rewriter. It is a structured pipeline:

```text
discover files
  -> resolve config
  -> parse CMake source
  -> classify commands using the command registry
  -> build formatted layout decisions
  -> emit text / diff / check result / in-place rewrite
```

That structure is what makes the tool safe and predictable — and what
separates it from a simple line-by-line formatter.

## Main Layers

### Parser

The parser is built on a `pest` PEG grammar. It understands:

- command invocations
- quoted, unquoted, and bracket arguments
- comments
- variable references
- generator expressions
- continuation lines

Comments are preserved as real syntax nodes throughout — they are never
stripped and guessed at later.

### Command Registry

The registry is what gives `cmakefmt` its semantic awareness.

Without it, every argument in:

```cmake
target_link_libraries(foo PUBLIC bar PRIVATE baz)
```

looks like a generic positional token. The registry knows that `PUBLIC` and
`PRIVATE` are not generic tokens — they start new argument groups. That
knowledge is what lets the formatter produce keyword-aware, correctly grouped
output instead of flattened token streams.

The registry is populated from two sources:

- built-in specs for CMake commands and supported module commands (audited through CMake 4.3.1)
- optional user config under `commands:`

### Formatter

Once the source is parsed and command shapes are known, the formatter converts
the AST into layout decisions using a Wadler-Lindig-style document model.

In practice, this means it can ask:

- can this stay on one line?
- if not, should it hang-wrap?
- if not, should it go fully vertical?

That is how `cmakefmt` gets stable, principled wrapping behavior instead of
ad-hoc line splitting that changes every time you touch a file.

### Config

Config resolution is layered — later layers only apply when earlier ones are
absent:

1. CLI overrides
2. explicit `--config-file` files, if any
3. nearest discovered `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`
4. home-directory fallback config
5. built-in defaults

Make the resolution process visible with:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --explain-config
```

### CLI Workflow Layer

The CLI is far more than a thin wrapper around `format_source`. It handles:

- recursive file discovery
- ignore files and Git-aware selection
- `--check`, `--diff`, and JSON reporting
- in-place rewrites
- partial and range formatting
- progress bars and parallel execution
- diagnostics and summary reporting

That workflow layer is a large part of what makes `cmakefmt` useful in real
repositories rather than just in toy examples.

## Diagnostics

When something goes wrong, `cmakefmt` tries hard to explain:

- which file failed
- where it failed
- what source text was involved
- what config was active
- what likely caused the failure

This is possible because the architecture keeps spans, config provenance, and
formatter decision context around long enough to report them meaningfully —
rather than discarding context as soon as each stage completes.

## Design Priorities

The codebase is intentionally optimized around:

- **correctness over cleverness** — no surprising heuristics
- **speed that is visible in day-to-day workflows** — 20× faster than `cmake-format` on real corpora
- **strong diagnostics** — failures explain themselves
- **configurability without scriptable config files** — powerful without being dangerous
- **maintainability of the grammar/registry/formatter pipeline** — easy to extend correctly

## Related Pages

- [Formatter Behavior](behavior.md)
- [Config Reference](config.md)
- [Library API](api.md)
- [Troubleshooting](troubleshooting.md)
