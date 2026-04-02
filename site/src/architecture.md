# Architecture

## Core Layers

- **Parser**
  - Pest grammar plus AST construction under `src/parser/`
- **Spec registry**
  - built-in and user-override command shapes under `src/spec/`
- **Formatter**
  - command layout, comment handling, barriers, and indentation under `src/formatter/`
- **Config**
  - defaults, TOML loading, and per-command overrides under `src/config/`
- **CLI**
  - file discovery, config resolution, debug mode, check/in-place modes in `src/main.rs`

## Operational Pipeline

```text
discover inputs
  -> resolve config
  -> parse source
  -> choose command forms from the registry
  -> format AST
  -> emit stdout / check diagnostics / in-place rewrite
```

## Diagnostics

Debug mode exists specifically to expose the parts of that pipeline that are
otherwise invisible: discovery, config sources, barriers, and formatting
decisions.

## Related Repository Docs

The source tree also carries deeper markdown notes for:

- architecture
- grammar
- performance
- roadmap/planning

This book is the user-facing published surface; the repository docs remain the
implementation-facing source material.
