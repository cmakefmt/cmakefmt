# Formatter Behavior

## Layout

- blank lines are preserved, bounded by `max_empty_lines`
- inline layout is preferred when it fits and the command shape permits it
- hanging-wrap is used for selected positional layouts
- vertical layout is used when arg-count and width heuristics are exceeded

## Comments

- standalone comments are preserved
- inline and trailing comments are preserved
- bracket comments are preserved
- line-comment reflow is opt-in via `markup.reflow_comments`

## Control Flow

Control-flow commands such as `if()`, `elseif()`, `else()`, `endif()`,
`foreach()`, `while()`, `function()`, `macro()`, and `block()` are indented as
structured blocks.

## Barriers And Fence Regions

```cmake
# cmakefmt: off
# cmakefmt: on
# cmake-format: off
# cmake-format: on
# ~~~
```

Disabled regions are passed through unchanged, even if they would not parse as
valid CMake on their own.

## Debug Mode

`--debug` reports file discovery, config sources, CLI overrides, barrier/fence
transitions, selected command forms, effective per-command thresholds, chosen
layout families, and a final changed-line summary.

## Known Differences From `cmake-format`

- output is intended to be stable and reasonable, not byte-identical
- compatibility options are still being expanded
- markup-aware comment handling is less mature than the core formatter path
