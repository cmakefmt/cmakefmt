# Formatter Behavior

This page describes what `cmakefmt` is trying to preserve, what it will change,
and how to reason about the output when you run it across a real codebase.

## Core Principles

`cmakefmt` tries to be:

- **safe**: formatting should not change the meaning of the file
- **idempotent**: formatting the same file twice should not keep changing it
- **predictable**: line wrapping and casing should follow explicit config
- **respectful of existing structure**: comments, disabled regions, and command
  shapes matter

## What `cmakefmt` Preserves

- comments and comment ordering
- bracket arguments and bracket comments
- disabled regions such as `# cmakefmt: off` / `# cmakefmt: on`
- command structure as understood by the built-in or user-supplied command spec
- blank-line separation, bounded by `max_empty_lines`
- parse-tree equivalence for formatted output on supported inputs

## What `cmakefmt` Intentionally Changes

- command name case when `command_case` is not `unchanged`
- keyword/flag case when `keyword_case` is not `unchanged`
- indentation and wrapping
- excess blank lines beyond the configured limit
- line-comment layout when markup/comment reflow options are enabled

## Layout Strategy

At a high level, `cmakefmt` prefers:

1. keep a call on one line when it fits
2. use a hanging-wrap layout when that stays readable
3. fall back to a more vertical layout when width and grouping heuristics are exceeded

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
thresholds.

## Blank Lines

`cmakefmt` preserves meaningful vertical separation, but it will clamp runaway
blank-line runs according to `format.max_empty_lines`.

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
elements that move through the formatter pipeline.

That matters because it lets `cmakefmt` preserve:

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

`cmakefmt` will keep the trailing comment attached to the relevant argument.

## Comment Markup

When markup handling is enabled, `cmakefmt` can treat some comments as lists,
fences, or rulers instead of opaque text.

Important knobs:

- `markup.enable_markup`
- `markup.reflow_comments`
- `markup.first_comment_is_literal`
- `markup.literal_comment_pattern`

If you want comments left almost entirely alone, keep `reflow_comments = false`.

## Control Flow And Blocks

Structured commands such as:

- `if` / `elseif` / `else` / `endif`
- `foreach` / `endforeach`
- `while` / `endwhile`
- `function` / `endfunction`
- `macro` / `endmacro`
- `block` / `endblock`

are treated as block constructs rather than generic flat calls. That affects
indentation and spacing around their parentheses.

Example with `space_before_control_paren = true`:

```cmake
if (WIN32)
  message(STATUS "Windows build")
endif ()
```

Without that option:

```cmake
if(WIN32)
  message(STATUS "Windows build")
endif()
```

## Disabled Regions And Fences

`cmakefmt` respects disabled regions and passes them through unchanged:

```cmake
# cmakefmt: off
set(SPECIAL_CASE   keep   this   exactly)
# cmakefmt: on
```

Supported markers include:

- `# cmakefmt: off`
- `# cmakefmt: on`
- `# cmake-format: off`
- `# cmake-format: on`
- `# ~~~`

This is the escape hatch for generated blocks, unusual macro DSLs, or legacy
sections you are not ready to normalize yet.

## Custom Commands

Custom commands are only formatted well if `cmakefmt` understands their syntax.

That is why the config has a `commands:` section. Once you teach the registry
what counts as:

- positional arguments
- standalone flags
- keyword sections and their arity

the formatter can group and wrap those commands much more intelligently than a
generic token list formatter could.

## Per-command Overrides

`per_command_overrides:` changes formatting knobs for one command name without
changing the command's syntax.

Use it when you want:

- a wider `line_width` for `message`
- different casing for one command
- different wrapping thresholds for a single noisy macro

Do **not** use it to describe a custom command's arguments. That belongs in
`commands:`.

## Range Formatting

When you use `--lines START:END`, `cmakefmt` formats only the selected ranges.
This is mainly for editor workflows and partial-file automation.

Important caveat:

- the selected range still lives inside a full CMake file
- surrounding structure still matters
- partial formatting is therefore best-effort rather than an isolated mini-file pass

## Debug Mode

`--debug` makes the hidden parts of the formatter visible. It reports:

- file discovery
- selected config files and CLI overrides
- barrier/fence transitions
- chosen command forms
- effective per-command layout thresholds
- chosen layout families
- changed-line summaries

When a formatting result surprises you, `--debug` is usually the first thing to run.

## Known Differences From `cmake-format`

`cmakefmt` is trying to be a practical replacement for `cmake-format`, not a
byte-for-byte clone.

That means:

- some outputs differ while still being valid and stable
- the config surface has been cleaned up in places
- workflow features are intentionally broader
- diagnostics are intentionally much more explicit

If you are comparing outputs during migration, judge by:

- readability
- stability
- semantic preservation
- ease of automation

not only by whether every wrapped line matches historical `cmake-format`
output exactly.
