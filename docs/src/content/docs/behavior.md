---
title: Formatter Behavior
description: How cmakefmt makes formatting decisions — safety, idempotency, and what it preserves versus rewrites.
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
3. try a compact grouped layout when `max_hanging_wrap_groups` allows it
4. fall back to a vertical layout when width and grouping thresholds are exceeded

`always_wrap` selects a vertical layout directly. A vertical layout can still
pack short argument lists on one line. See the [alignment options](/config/#wrap_after_first_arg_align)
for how command-body indentation and keyword-value continuations combine.

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

## Multiline Literal Whitespace

Whitespace inside quoted strings and bracket arguments is part of their value.
Tab indentation changes the command's indentation, not spaces inside those
values. This also applies when `wrap_after_first_arg` keeps a multiline first
argument beside the command name, or when a later argument spans multiple lines.

For example, with `use_tabs: true` and `wrap_after_first_arg: true`, the two
spaces before `b` remain two spaces:

```cmake
message("a
  b")
```

CRLF input is normalized internally before the configured output line endings
are applied once. Repeated formatting must not accumulate carriage returns
inside a literal. See [`line_ending`](/config/#line_ending) for the available
styles and [Range Formatting](#range-formatting) for preservation of unselected
source bytes.

## Comments

Comments are never discarded and re-inserted later. They are tracked as real
syntax nodes throughout the entire formatter pipeline.

That distinction matters. It means `cmakefmt` can reliably preserve:

- standalone comments above a command
- standalone comments between arguments
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

With the default `format.argument_comment_style: preserve`, `cmakefmt` keeps
the inline comment attached to the relevant argument where it fits, and keeps
originally standalone argument comments on separate lines. Set
`format.argument_comment_style: standalone` to put all argument comments on
separate lines, including those originally inline.

See [`argument_comment_style`](/config/#argument_comment_style) for both modes.

Multiline bracket-comment continuation lines retain their existing whitespace.
Only the opening line receives formatter indentation; continuation indentation
is not added again on each pass. This applies to standalone, argument-list,
and trailing bracket comments, independently of `enable_markup`:

```cmake
set(A x) #[=[tail
text]=]
```

The `text]=]` line stays as written rather than gaining spaces on each run.


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

With `separate_ctrl_name_with_space = true`:

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
sections you are not ready to normalize yet. Markers inside command arguments
or bracket comments are literal content and do not toggle formatting.

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

The variable name stays attached. By default, remaining items align just
past the opening parenthesis:

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

To keep the variable name attached but indent remaining arguments by one
normal indentation level, set `format.wrap_after_first_arg_align: default-indent`.

See [`wrap_after_first_arg`](/config/#wrap_after_first_arg) and
[`wrap_after_first_arg_align`](/config/#wrap_after_first_arg_align) in the
config reference for the full option documentation.

## Trailing Comments

With `format.argument_comment_style: preserve` (the default), inline comments
(``# text``) that follow an argument stay attached to that argument when the
command wraps. The comment and argument are kept
on the same line as long as the combined width fits within `line_width`.
The `standalone` mode instead moves these comments onto separate lines.
Neither mode attaches originally standalone comments to a preceding argument.
This option does not affect comments after a command's closing `)`.

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

### Comments on the `set()` variable name

When `wrap_after_first_arg` keeps the variable name on the `set(` line,
an inline comment stays attached:

```cmake
set(foobarbaz # explanation of this variable
    value_one value_two value_three)
```

### Comments in `CACHE` sections

A comment within a `CACHE` section is reflowed (when `enable_markup`
is on) and placed on its own line:

```cmake
set(CMAKE_BUILD_TYPE "Release"
    CACHE
      STRING
      # This comment explains why we default to Release
      # and spans multiple lines after reflow.
      "Build mode for performance." FORCE)
```

### Comments on commands

A short comment after the closing parenthesis stays inline:

```cmake
set(FOO bar) # explanation of this variable
```

### Comments on control flow

Comments on `if()`, `foreach()`, and similar stay inline:

```cmake
if(CONDITION) # check this before proceeding
  message(STATUS "hello")
endif()
```

### Comments do not force wrapping

The presence of a trailing comment does not by itself force a command
into a vertical layout. The layout decision (inline, hanging, or
vertical) is made independently based on line width and wrapping
thresholds. The comment is then rendered within the chosen layout.

### Comment reflow

When `markup.enable_markup` is `true` (the default), long comments
that exceed `line_width` are reflowed with continuation lines aligned
to the `#`:

```cmake
# Before:
set(FOO bar) # this trailing comment is deliberately long enough to exceed the default line width and demonstrate reflow clearly

# After (enable_markup: true):
set(FOO bar) # this trailing comment is deliberately long enough to
             # exceed the default line width and demonstrate reflow
             # clearly

# After (enable_markup: false):
set(FOO bar) # this trailing comment is deliberately long enough to exceed the default line width and demonstrate reflow clearly
```

To disable comment reflow entirely, set `markup.enable_markup: false`.

## Opting Files In Gradually

Use `cmakefmt --require-pragma --check .` to process only files containing an
opt-in marker:

```cmake
# cmakefmt: enable
```

`# fmt: enable` and `# cmake-format: enable` are also accepted. Files without
an opt-in marker are skipped. Disabled regions still apply within opted-in files.

## Range Formatting

`--lines START:END` selects complete commands intersecting the inclusive,
1-based **source** line range. Repeat `--lines` to select disjoint ranges.
Changing one argument can reformat its entire command, including attached
trailing comments. A selected command can grow or shrink beyond the requested
lines without changing any other command.

All commands are read to retain block indentation, but unselected source bytes
are copied verbatim. Standalone comments, blank lines, and disabled regions are
preserved, even if they contain legacy formatting. Selecting only standalone
comments, whitespace, or lines past EOF makes no changes. Line-ending settings
apply inside selected commands; they do not rewrite the rest of the file.

`--verify` checks the final combined output. Range runs bypass the whole-file
cache so cached formatting cannot widen their scope. LSP range formatting uses
the same command selection and retains document context. Its end position is
exclusive: an end at column zero excludes that line. An empty selection formats
the command on the cursor's line.

Range formatting rejects a replacement if reparsing it would change comment
ownership, such as aligning a trailing `#` with an untouched standalone comment
and turning that comment into a continuation. This check also applies without
`--verify`, whose semantic comparison excludes comments. Syntax errors in
unselected enabled source can still prevent a range request.

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

For structural questions — "how does the parser see this file?" or "why
is this token treated as a keyword?" — use the tree dump commands:

```bash
cmakefmt dump ast CMakeLists.txt    # raw parser AST
cmakefmt dump parse CMakeLists.txt  # spec-resolved tree
```

See [Parse Tree Dump](/cli/#parse-tree-dump) in the CLI reference for
full details and example output.

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

**Comment reflow.** `markup.enable_markup` is `true` by default, so eligible
comments are reflowed to fit the configured line width. The first comment block
is preserved literally by default (`markup.first_comment_is_literal: true`).
Set `markup.enable_markup: false` to disable markup processing and reflow.

**Config key names.** Several config keys were renamed for clarity. Any key
`cmakefmt` does not recognise will produce a fast-fail error, not a silent
no-op. The full renaming table is in [Config Reference](/config/#legacy-config-names).

When comparing outputs during migration, judge by readability, stability,
semantic preservation, and ease of automation — not solely by whether every
wrapped line matches historical `cmake-format` output exactly.
