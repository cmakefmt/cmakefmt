---
title: Formatter Behavior
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

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

Comments are never discarded and re-inserted later. They are tracked as real
syntax nodes throughout the entire formatter pipeline.

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
- `markup.first_comment_is_literal`
- `markup.literal_comment_pattern`

To leave comments almost entirely alone, set `enable_markup: false`.

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
- `# fmt: off` / `# fmt: on`
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

## `set()` Formatting

`set()` is the most common CMake command and has several distinct usage
patterns. `cmakefmt` handles each one with a specific rule: **the variable
name always stays on the `set(` line**. This is enabled by the built-in
`wrap_after_first_arg` layout hint on the `set` command spec.

### Simple and short values

When everything fits on one line, it stays inline:

```cmake
set(FOO bar)
set(FOO a b c)
set(FOO "value" PARENT_SCOPE)
set(ENV{FOO} "value")
set(FOO)
```

### Lists that wrap

The variable name stays attached. Remaining items are aligned to the
open parenthesis:

```cmake
set(HEADERS
    header_a.h
    header_b.h
    header_c.h
    header_d.h)
```

### Cached variables

When everything fits, it stays on one or two lines:

```cmake
set(FOO "default" CACHE STRING "A description" FORCE)

set(CMAKE_BUILD_TYPE "Release"
    CACHE STRING "Build mode for performance." FORCE)
```

When the `CACHE` section is too long, it wraps with `STRING` and the
description nested under `CACHE`:

```cmake
set(CMAKE_BUILD_TYPE "Release"
    CACHE
      STRING "A very long description that doesn't fit on one line"
      FORCE)
```

### Comments on the variable name

Inline comments stay attached to the variable name:

```cmake
set(MY_VAR # explanation of the variable
    value_one value_two value_three)
```

### Overriding the behavior

To disable `wrap_after_first_arg` for `set()`:

```yaml
per_command_overrides:
  set:
    wrap_after_first_arg: false
```

This reverts to the standard vertical layout where everything wraps
below the opening parenthesis:

```cmake
set(
  MY_VAR
  value_one
  value_two)
```

See [`wrap_after_first_arg`](/config/#wrap_after_first_arg) in the
config reference for the full option documentation.

## Trailing Comments

Inline comments (``# text``) that follow an argument stay attached to
that argument when the command wraps. The comment and argument are kept
on the same line as long as the combined width fits within `line_width`.

```cmake
target_link_libraries(
  mylib
  PUBLIC
    dep1 # first dependency
    dep2 # second dependency
    dep3 # third dependency)
```

If a trailing comment would exceed `line_width`, it moves to its own
line at the current indentation:

```cmake
target_link_libraries(
  mylib
  PUBLIC
    some_very_long_dependency_name
    # This comment is too long to fit after the argument
    another_dependency)
```

### Comments on commands

A comment after the closing parenthesis is preserved as-is:

```cmake
set(FOO bar) # explanation of this variable
```

### Comments do not force wrapping

The presence of a trailing comment does not by itself force a command
into a vertical layout. The layout decision (inline, hanging, or
vertical) is made independently based on line width and wrapping
thresholds. The comment is then rendered within the chosen layout.

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
- the config surface has been cleaned up in places (see [Config Reference](/config/)
  for the old-to-new key name mapping)
- workflow features are intentionally broader
- diagnostics are intentionally much more explicit

### Output differences you may notice

**Wrapping thresholds.** `cmakefmt` uses a principled pretty-printing algorithm
to decide layouts. The exact line at which a call wraps can differ from
`cmake-format`'s heuristics, even with identical config values. The result is
still correct and idempotent — just not always output-identical.

**Keyword grouping.** When `cmakefmt` knows a command's structure (via the
built-in registry or a `commands:` entry), it groups keyword sections
deliberately. `cmake-format` without a matching spec entry would often treat
the same tokens as undifferentiated positional arguments and produce a flatter
layout.

**Comment reflow.** By default, `cmakefmt` preserves comments without
modification. If you want comments reflowed to fit within the configured line
width, enable `markup.enable_markup: true`.

**Config key names.** Several config keys were renamed for clarity. Any key
`cmakefmt` does not recognise will produce a fast-fail error, not a silent
no-op. The full renaming table is in [Config Reference](config.md#old-draft-key-names).

When comparing outputs during migration, judge by readability, stability,
semantic preservation, and ease of automation — not solely by whether every
wrapped line matches historical `cmake-format` output exactly.
