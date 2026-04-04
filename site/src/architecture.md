# Architecture

This page is the user-facing overview of how `cmakefmt` works. For the deeper
implementation notes, see the repository's `docs/ARCHITECTURE.md`.

## Mental Model

`cmakefmt` is not a regex-based text rewriter. It works as a structured
pipeline:

```text
discover files
  -> resolve config
  -> parse CMake source
  -> classify commands using the command registry
  -> build formatted layout decisions
  -> emit text / diff / check result / in-place rewrite
```

That structure is what makes the tool safer and more predictable than simple
line-based rewriting.

## Main Layers

## Parser

The parser is built on a `pest` grammar. It understands:

- command invocations
- quoted, unquoted, and bracket arguments
- comments
- variable references
- generator expressions
- continuation lines

Comments are preserved as real syntax nodes, not stripped out and guessed at later.

## Command Registry

The registry teaches `cmakefmt` what a command means structurally.

For example, it knows that in:

```cmake
target_link_libraries(foo PUBLIC bar PRIVATE baz)
```

`PUBLIC` and `PRIVATE` are not generic positional tokens. They start new
argument groups. The registry is what lets the formatter produce readable,
keyword-aware output instead of flattening everything into a token stream.

The registry comes from two places:

- built-in specs for CMake commands and supported module commands
- optional user config under `commands:`

## Formatter

Once the source is parsed and command shapes are known, the formatter turns the
AST into layout decisions using a Wadler-Lindig-style document model.

In practical terms, this means it can ask:

- can this stay on one line?
- if not, should it hang-wrap?
- if not, should it go vertical?

That is how `cmakefmt` gets stable wrapping behavior instead of ad-hoc line splitting.

## Config

Config resolution is layered:

1. CLI overrides
2. explicit `--config-file` files, if any
3. nearest discovered `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`
4. home-directory fallback config
5. built-in defaults

The CLI can also explain this process with:

- `--show-config-path`
- `--show-config`
- `--explain-config`

## CLI Workflow Layer

The CLI is more than just a thin wrapper around `format_source`.

It also handles:

- recursive file discovery
- ignore files and Git-aware selection
- `--check`, `--diff`, and JSON reporting
- in-place rewrites
- partial/range formatting
- progress bars and parallel execution
- diagnostics and summary reporting

That workflow layer is a big part of what makes `cmakefmt` useful in real repositories.

## Diagnostics

When something goes wrong, `cmakefmt` tries to explain:

- which file failed
- where it failed
- what source text was involved
- what config was used
- what likely caused the failure

That is why the architecture keeps spans, config provenance, and formatter
decision context around long enough to report them meaningfully.

## Design Priorities

The codebase is intentionally optimized around:

- correctness over cleverness
- speed that is visible in day-to-day workflows
- strong diagnostics
- configurability without scriptable config files
- maintainability of the grammar/registry/formatter pipeline

## Related Pages

- [Formatter Behavior](behavior.md)
- [Config Reference](config.md)
- [Library API](api.md)
- [Troubleshooting](troubleshooting.md)
