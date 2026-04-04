# Formatter Behavior

What `cmakefmt` preserves, what it intentionally changes, and how to reason
about the output when you run it across a real codebase.

## Core Principles

`cmakefmt` is designed to be:

- **safe**: formatting must never change the meaning of the file
- **idempotent**: formatting the same file twice must produce the same result
- **predictable**: line wrapping and casing follow explicit config, not heuristics you have to reverse-engineer
- **respectful of structure**: comments, disabled regions, and command shapes are all first-class

## What `cmakefmt` Preserves

- comments and comment ordering
- bracket arguments and bracket comments
- disabled regions (`# cmakefmt: off` / `# cmakefmt: on`)
- command structure as defined by the built-in or user-supplied command spec
- blank-line separation, bounded by `max_empty_lines`
- parse-tree equivalence for formatted output on supported inputs

## What `cmakefmt` Intentionally Changes

- command name case when `command_case` is not `unchanged`
- keyword and flag case when `keyword_case` is not `unchanged`
- indentation and wrapping
- blank-line runs that exceed the configured limit
- line-comment layout when markup or comment-reflow options are enabled

## Layout Strategy

`cmakefmt` tries the simplest layout first and only escalates when necessary:

1. keep a call on one line when it fits
2. use a hanging-wrap layout when that stays readable
3. fall back to a more vertical layout when width and grouping thresholds are exceeded

### Compact Layout

Input:

```cmake
target_link_libraries(foo PUBLIC bar)
```

Output:

```cmake
target_link_libraries(foo PUBLIC bar)
```

### Wrapped Layout

Input:

```cmake
target_link_libraries(foo PUBLIC very_long_dependency_name another_dependency)
```

Typical output:

```cmake
target_link_libraries(
  foo
  PUBLIC
    very_long_dependency_name
    another_dependency)
```

The exact shape depends on the command spec, line width, and wrapping
thresholds in your config.

## Blank Lines

`cmakefmt` preserves meaningful vertical separation, but clamps runaway
blank-line gaps according to `format.max_empty_lines`.

Input:

```cmake
project(example)



add_library(foo foo.cc)
```

Output with `max_empty_lines = 1`:

```cmake
project(example)

add_library(foo foo.cc)
```

## Comments

Comments are not stripped and reattached later. They are first-class parsed
elements that move through the entire formatter pipeline.

That distinction matters. It means `cmakefmt` can reliably preserve:

- standalone comments above a command
- inline argument-list comments
- trailing same-line comments
- bracket comments

Example:

```cmake
target_sources(foo
  PRIVATE
    foo.cc # platform-neutral
    bar.cc)
```

`cmakefmt` keeps the trailing comment attached to the relevant argument.

## Comment Markup

When markup handling is enabled, `cmakefmt` can recognize and treat some
comments as lists, fences, or rulers rather than opaque text.

The key knobs:

- `markup.enable_markup`
- `markup.reflow_comments`
- `markup.first_comment_is_literal`
- `markup.literal_comment_pattern`

To leave comments almost entirely alone, keep `reflow_comments = false`.

## Control Flow And Blocks

Structured commands — `if`/`elseif`/`else`/`endif`, `foreach`/`endforeach`,
`while`/`endwhile`, `function`/`endfunction`, `macro`/`endmacro`,
`block`/`endblock` — are treated as block constructs rather than flat calls.
This affects indentation and spacing around their parentheses.

With `space_before_control_paren = true`:

```cmake
if (WIN32)
  message(STATUS "Windows build")
endif ()
```

Without it:

```cmake
if(WIN32)
  message(STATUS "Windows build")
endif()
```

## Disabled Regions And Fences

Need to protect a block from formatting? Use a disabled region:

```cmake
# cmakefmt: off
set(SPECIAL_CASE   keep   this   exactly)
# cmakefmt: on
```

All of the following markers work:

- `# cmakefmt: off` / `# cmakefmt: on`
- `# cmake-format: off` / `# cmake-format: on`
- `# ~~~`

This is the escape hatch for generated blocks, unusual macro DSLs, or legacy
sections you are not ready to normalize yet.

## Custom Commands

Custom commands format well only when `cmakefmt` understands their structure.
That is what `commands:` in your config is for. Once you tell the registry what
counts as positional arguments, standalone flags, and keyword sections, the
formatter groups and wraps those commands intelligently — instead of treating
every token as an undifferentiated lump.

## Per-command Overrides

`per_command_overrides:` changes formatting knobs for a single command name
without touching its argument structure.

Use it when you want:

- a wider `line_width` for `message`
- different casing for one specific command
- different wrapping thresholds for a single noisy macro

Do **not** use it to describe a command's argument structure. That belongs in
`commands:`.

## Range Formatting

`--lines START:END` formats only selected line ranges. This is mainly for
editor workflows and partial-file automation.

Important: the selected range still lives inside a full CMake file. Surrounding
structure still applies. Partial formatting is best-effort, not an isolated
mini-file pass.

## Debug Mode

When a formatting result surprises you, `--debug` is the first thing to reach
for. It surfaces everything the formatter normally keeps to itself:

- file discovery
- selected config files and CLI overrides
- barrier and fence transitions
- chosen command forms
- effective per-command layout thresholds
- chosen layout families
- changed-line summaries

## Known Differences From `cmake-format`

`cmakefmt` is a practical replacement for `cmake-format`, not a byte-for-byte
clone. That means:

- some outputs differ while still being valid and stable
- the config surface has been cleaned up in places
- workflow features are intentionally broader
- diagnostics are intentionally much more explicit

When comparing outputs during migration, judge by readability, stability,
semantic preservation, and ease of automation — not solely by whether every
wrapped line matches historical `cmake-format` output exactly.
