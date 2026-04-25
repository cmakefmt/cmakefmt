---
title: Config Reference
description: Every cmakefmt config option explained — YAML and TOML syntax, discovery order, and per-command overrides.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Everything you need to know to tune `cmakefmt` for your project.

The short version:

- configuration files may be YAML or TOML
- YAML is the recommended default for hand-edited configs
- custom command syntax goes under `commands:`
- command-specific layout and style tweaks go under `per_command_overrides:`

## Config Discovery Order

For a given target file, `cmakefmt` resolves config by layering sources in this
order — higher layers win over lower ones:

1. **CLI flag overrides** (`--line-width`, `--tab-size`, `--command-case`, etc.) — always win, regardless of what any config file says
2. **Explicit `--config-file <PATH>` files**, if provided — later files override earlier ones
3. **The nearest `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`** found by walking upward from the target file
4. **`~/.cmakefmt.yaml`, `~/.cmakefmt.yml`, or `~/.cmakefmt.toml`** — home-directory fallback
5. **Built-in defaults**

If multiple supported config filenames exist in the same directory, YAML is
preferred over TOML.

When you want to see exactly what happened:

```bash
cmakefmt config path src/CMakeLists.txt
cmakefmt config show src/CMakeLists.txt
cmakefmt config explain
```

## Recommended Starter File

YAML is the recommended user-facing format:

```yaml
format:
  line_width: 80
  tab_size: 2
  command_case: lower
  keyword_case: upper
```

Generate the full starter template with:

```bash
cmakefmt config dump > .cmakefmt.yaml
```

If you prefer TOML:

```bash
cmakefmt config dump --format toml > .cmakefmt.toml
```

## Table Of Contents

- [Config Discovery Order](#config-discovery-order)
- [Recommended Starter File](#recommended-starter-file)
- [Table Of Contents](#table-of-contents)
- [Defaults](#defaults)
- [Format Options](#format-options)
  - [`disable`](#disable)
  - [`line_ending`](#line_ending)
  - [`line_width`](#line_width)
  - [`tab_size`](#tab_size)
  - [`use_tabs`](#use_tabs)
  - [`fractional_tab_policy`](#fractional_tab_policy)
  - [`max_empty_lines`](#max_empty_lines)
  - [`max_hanging_wrap_lines`](#max_hanging_wrap_lines)
  - [`max_hanging_wrap_positional_args`](#max_hanging_wrap_positional_args)
  - [`max_hanging_wrap_groups`](#max_hanging_wrap_groups)
  - [`max_rows_cmdline`](#max_rows_cmdline)
  - [`always_wrap`](#always_wrap)
  - [`require_valid_layout`](#require_valid_layout)
  - [`wrap_after_first_arg`](#wrap_after_first_arg)
  - [`continuation_align`](#continuation_align)
  - [`enable_sort`](#enable_sort)
  - [`autosort`](#autosort)
  - [`dangle_parens`](#dangle_parens)
  - [`dangle_align`](#dangle_align)
  - [`min_prefix_length`](#min_prefix_length)
  - [`max_prefix_length`](#max_prefix_length)
  - [`space_before_control_paren`](#space_before_control_paren)
  - [`space_before_definition_paren`](#space_before_definition_paren)
- [Casing Options](#casing-options)
  - [`command_case`](#command_case)
  - [`keyword_case`](#keyword_case)
- [Markup Options](#markup-options)
  - [`enable_markup`](#enable_markup)
  - [`first_comment_is_literal`](#first_comment_is_literal)
  - [`literal_comment_pattern`](#literal_comment_pattern)
  - [`bullet_char`](#bullet_char)
  - [`enum_char`](#enum_char)
  - [`fence_pattern`](#fence_pattern)
  - [`ruler_pattern`](#ruler_pattern)
  - [`hashruler_min_length`](#hashruler_min_length)
  - [`canonicalize_hashrulers`](#canonicalize_hashrulers)
  - [`explicit_trailing_pattern`](#explicit_trailing_pattern)
- [`commands:` vs `per_command_overrides:` — Which One Do I Need?](#commands-vs-per_command_overrides--which-one-do-i-need)
- [Per-command Overrides](#per-command-overrides)
- [Custom Command Specs](#custom-command-specs)
  - [Before and after](#before-and-after)
  - [Spec fields](#spec-fields)
    - [`pargs`](#pargs)
    - [`flags`](#flags)
    - [`kwargs`](#kwargs)
    - [`layout`](#layout)
  - [Discriminated commands](#discriminated-commands)
  - [A larger example](#a-larger-example)
- [Old Draft Key Names](#old-draft-key-names)
- [Related Reading](#related-reading)

## Defaults

```yaml
format:
  disable: false
  line_ending: unix
  line_width: 80
  tab_size: 2
  use_tabs: false
  fractional_tab_policy: use-space
  max_empty_lines: 1
  max_hanging_wrap_lines: 2
  max_hanging_wrap_positional_args: 6
  max_hanging_wrap_groups: 2
  max_rows_cmdline: 2
  always_wrap: []
  require_valid_layout: false
  wrap_after_first_arg: false
  enable_sort: false
  autosort: false
  dangle_parens: false
  dangle_align: prefix
  min_prefix_length: 4
  max_prefix_length: 10
  space_before_control_paren: false
  space_before_definition_paren: false
  command_case: lower
  keyword_case: upper

markup:
  enable_markup: true
  first_comment_is_literal: true
  literal_comment_pattern: ""
  bullet_char: "*"
  enum_char: "."
  fence_pattern: "^\\s*[`~]{3}[^`\\n]*$"
  ruler_pattern: "^[^\\w\\s]{3}.*[^\\w\\s]{3}$"
  hashruler_min_length: 10
  canonicalize_hashrulers: true
  explicit_trailing_pattern: "#<"
```

## Format Options

### `disable`

Disable formatting entirely. When `true`, `cmakefmt` returns the source
unchanged — no layout changes, no casing normalization, nothing.

```yaml
format:
  disable: true
```

Useful as a temporary escape hatch or for opting individual files out via a
project-local config.

### `line_ending`

Output line-ending style.

Allowed values:

- `unix` — LF (`\n`). The default.
- `windows` — CRLF (`\r\n`).
- `auto` — detect from the input source; if the input contains any `\r\n`,
  use CRLF; otherwise use LF.

```yaml
format:
  line_ending: windows
```

The formatter normalizes line endings internally to LF. This option controls
only the final output.

### `line_width`

Target maximum output width before wrapping is attempted.

```yaml
format:
  line_width: 100
```

Raise this if your project prefers wider CMake calls.

### `tab_size`

Indent width in spaces when `use_tabs` is `false`.

```yaml
format:
  tab_size: 4
```

### `use_tabs`

Use tab characters for indentation instead of spaces.

```yaml
format:
  use_tabs: true
```

This affects leading indentation only. Internal alignment rules use the
configured indentation unit but are not otherwise changed.

### `fractional_tab_policy`

Controls what happens to fractional (sub-tab-stop) indentation when
`use_tabs` is `true`.

Allowed values:

- `use-space` — leave remaining spaces as-is (utf-8 0x20). The default.
- `round-up` — promote the remaining spaces to a full tab character
  (utf-8 0x09), shifting the column to the next tab stop.

```yaml
format:
  fractional_tab_policy: round-up
```

Only relevant when `use_tabs: true`. Has no effect when `use_tabs: false`.

### `max_empty_lines`

Maximum number of consecutive blank lines to preserve.

```yaml
format:
  max_empty_lines: 1
```

Blank-line runs that exceed this limit are reduced to the configured maximum. Intentional vertical separation is preserved; excessive gaps are removed.

### `max_hanging_wrap_lines`

Maximum number of lines a hanging-wrap layout may consume before the formatter
falls back to a more vertical layout.

```yaml
format:
  max_hanging_wrap_lines: 2
```

Lower values cause more commands to fall back to fully vertical layout.

### `max_hanging_wrap_positional_args`

Maximum positional arguments to keep in a hanging-wrap layout before falling
back to a more vertical layout.

```yaml
format:
  max_hanging_wrap_positional_args: 6
```

Most noticeable on commands with long source or header lists.

### `max_hanging_wrap_groups`

Maximum number of keyword/flag subgroups to keep in a hanging-wrap layout.

```yaml
format:
  max_hanging_wrap_groups: 2
```

Lower this to format keyword-heavy commands with vertical layout more readily.

### `max_rows_cmdline`

Maximum number of rows a hanging-wrap positional group may consume before the
formatter rejects the hanging layout and forces nesting.

```yaml
format:
  max_rows_cmdline: 2
```

This is a second threshold that works alongside `max_hanging_wrap_lines`.
Where `max_hanging_wrap_lines` limits the total line count packed by the token
packer, `max_rows_cmdline` limits how many rows the result may span before
being rejected outright.

### `always_wrap`

A list of command names that the formatter must always lay out vertically,
regardless of line width or argument count.

```yaml
format:
  always_wrap:
    - target_link_libraries
    - target_sources
```

Commands in this list skip the inline and hanging-wrap layout attempts and go
directly to vertical layout. This is also configurable per-command via
`layout.always_wrap` in a custom command spec under `commands:`.

### `require_valid_layout`

When `true`, return an error if any formatted output line exceeds `line_width`.
The formatter does not guarantee that every line fits — long unbreakable tokens
or deeply nested commands can exceed the limit — and this option makes such
cases visible.

```yaml
format:
  require_valid_layout: true
```

Useful in CI to enforce a strict line-width contract. The error message
includes the line number, actual width, and configured limit.

### `wrap_after_first_arg`

When wrapping a command across multiple lines, keep the first positional
argument on the command line and align continuation lines to the open
parenthesis. Default: `false`.

```yaml
format:
  wrap_after_first_arg: true
```

When enabled globally, all commands that wrap will keep their first argument
attached:

```cmake
# wrap_after_first_arg: true
target_link_libraries(mylib
                      PUBLIC dep1 dep2
                      PRIVATE dep3 dep4)

# wrap_after_first_arg: false (default)
target_link_libraries(
  mylib
  PUBLIC dep1 dep2
  PRIVATE dep3 dep4)
```

**Built-in default for `set()`:** The `set` command has `wrap_after_first_arg`
enabled in its built-in spec, so the variable name always stays on the
`set(` line regardless of the global setting:

```cmake
# set() always keeps the variable name attached
set(CMAKE_BUILD_TYPE "Release"
    CACHE STRING "Build mode." FORCE)

set(SOURCES
    a.cpp
    b.cpp
    c.cpp)
```

Override per-command via `per_command_overrides`:

```yaml
per_command_overrides:
  set:
    wrap_after_first_arg: false
```

Or enable for a specific custom command in the spec:

```yaml
commands:
  my_cmd:
    pargs: 1
    layout:
      wrap_after_first_arg: true
    kwargs:
      SOURCES:
        nargs: "+"
```

### `continuation_align`

How to indent continuation lines when a wrapped keyword section (like a
subkwarg plus its values) overflows `line_width`. Two modes:

- `under-first-value` (default) — the continuation aligns under the
  column of the first value after the keyword (the cmake-format
  hanging-indent style). Keeps continuation visually grouped with the
  keyword, removing the ambiguity between a continuation and a new
  sibling subkwarg.
- `same-indent` — the continuation wraps at the same indent as the
  keyword itself. Consistent with how cmakefmt wraps flat-list
  sections and positional arg lists elsewhere.

```yaml
format:
  continuation_align: same-indent
```

The knob only affects subkwarg groups inside structural kwarg sections
(e.g. `PATTERN … PERMISSIONS …` inside `install(DIRECTORY)` or
`RUNTIME COMPONENT …` inside `install(TARGETS)`); flat-list sections
like `PUBLIC` in `target_link_libraries` are unaffected.

With a long `PERMISSIONS` value list inside a `PATTERN` subgroup:

```cmake
# under-first-value (default) — continuation aligned under OWNER_EXECUTE's column
install(DIRECTORY src/
  DESTINATION include
  PATTERN *.h
    PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ
                GROUP_EXECUTE GROUP_READ)

# same-indent — continuation at the subkwarg's own indent
install(DIRECTORY src/
  DESTINATION include
  PATTERN *.h
    PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ
    GROUP_EXECUTE GROUP_READ)
```

Override per-command via `per_command_overrides`:

```yaml
per_command_overrides:
  install:
    continuation_align: same-indent
```

Or in a custom spec via `layout.continuation_align`.

### `enable_sort`

Sort arguments in keyword sections marked `sortable` in the command spec.
Sorting is lexicographic and case-insensitive. Default: `false`.

```yaml
format:
  enable_sort: true
```

Mark a keyword section as sortable in a custom command spec:

```yaml
commands:
  my_cmd:
    kwargs:
      SOURCES:
        nargs: "+"
        sortable: true
```

### `autosort`

Heuristically sort keyword sections even without an explicit `sortable`
annotation. A section is considered sortable if all its arguments are simple
unquoted tokens (no variables, generator expressions, or quoted strings).
Requires `enable_sort: true` to have any effect. Default: `false`.

```yaml
format:
  enable_sort: true
  autosort: true
```

### `dangle_parens`

Place the closing `)` on its own line when a call wraps.

```yaml
format:
  dangle_parens: true
```

Effect:

```cmake
target_link_libraries(
  foo
  PUBLIC
    bar
    baz
)
```

### `dangle_align`

Alignment strategy for a dangling closing `)`.

Allowed values:

- `prefix`
- `open`
- `close`

```yaml
format:
  dangle_align: prefix
```

### `min_prefix_length`

Lower heuristic bound used when deciding between compact and wrapped layouts.

```yaml
format:
  min_prefix_length: 4
```

Leave this alone unless you are deliberately tuning layout behavior.

### `max_prefix_length`

Upper heuristic bound used when deciding between compact and wrapped layouts.

```yaml
format:
  max_prefix_length: 10
```

Like `min_prefix_length`, this is primarily for advanced layout tuning and rarely needs adjustment.

### `space_before_control_paren`

Insert a space before `(` for control-flow commands such as `if`, `foreach`,
and `while`.

```yaml
format:
  space_before_control_paren: true
```

Effect:

```cmake
if (WIN32)
  message(STATUS "Windows")
endif ()
```

### `space_before_definition_paren`

Insert a space before `(` for `function` and `macro` definitions.

```yaml
format:
  space_before_definition_paren: true
```

Effect:

```cmake
function (my_helper arg)
  ...
endfunction ()
```

## Casing Options

### `command_case`

Controls the casing of command names.

Allowed values:

- `lower`
- `upper`
- `unchanged`

```yaml
format:
  command_case: lower
```

### `keyword_case`

Controls the casing of recognized keywords and flags.

Allowed values:

- `lower`
- `upper`
- `unchanged`

```yaml
format:
  keyword_case: upper
```

Example — with `command_case: lower` and `keyword_case: upper`:

```cmake
target_link_libraries(foo PUBLIC bar)
```

stays:

```cmake
target_link_libraries(foo PUBLIC bar)
```

With `command_case: upper` and `keyword_case: lower`:

```cmake
TARGET_LINK_LIBRARIES(foo public bar)
```

## Markup Options

### `enable_markup`

Enable comment-markup awareness.

```yaml
markup:
  enable_markup: true
```

When enabled, the formatter reflows plain line comments to fit within the
configured line width and can recognize lists, fences, and rulers inside
comments rather than treating them as opaque text.

### `first_comment_is_literal`

Preserve the first comment block in a file without any reflowing or markup
processing.

```yaml
markup:
  first_comment_is_literal: true
```

Useful for license headers or hand-crafted introductory comments that must stay
exactly as written.

### `literal_comment_pattern`

Regex for comments that should never be reflowed.

```yaml
markup:
  literal_comment_pattern: "^\\s*NOTE:"
```

Use this for project-specific comment conventions that must stay untouched.

### `bullet_char`

Preferred bullet character when normalizing markup lists.

```yaml
markup:
  bullet_char: "*"
```

### `enum_char`

Preferred punctuation for numbered lists when normalizing markup.

```yaml
markup:
  enum_char: "."
```

### `fence_pattern`

Regex describing fenced literal comment blocks.

```yaml
markup:
  fence_pattern: "^\\s*[`~]{3}[^`\\n]*$"
```

Keep the default unless your project has a strong house style.

### `ruler_pattern`

Regex describing ruler-style comments that should be treated specially.

```yaml
markup:
  ruler_pattern: "^[^\\w\\s]{3}.*[^\\w\\s]{3}$"
```

### `hashruler_min_length`

Minimum length before a hash-only line is treated as a ruler.

```yaml
markup:
  hashruler_min_length: 10
```

### `canonicalize_hashrulers`

Normalize hash-ruler comments when markup handling is enabled.

```yaml
markup:
  canonicalize_hashrulers: true
```

If your project uses decorative comment rulers and wants them normalized
consistently, keep this enabled.

### `explicit_trailing_pattern`

A regex pattern that identifies inline comments as _explicitly trailing_ their
preceding argument. When a comment matches this pattern it is rendered on the
same line as the preceding token rather than on its own indented line.

```yaml
markup:
  explicit_trailing_pattern: "#<"
```

The default `#<` means that inline comments starting with `#<` are treated as
trailing the immediately preceding argument.

Example — given `explicit_trailing_pattern: "#<"`:

```cmake
target_sources(
  mylib
  PRIVATE
    src/foo.cpp #< main module
    src/bar.cpp #< helper
)
```

Without this option the `#<` comments would each appear on their own line.

Set to an empty string to disable explicit trailing comment detection entirely.

## `commands:` vs `per_command_overrides:` — Which One Do I Need?

These two config sections are easy to confuse. The short rule:

| Question | Answer |
|---|---|
| "The formatter doesn't know what `SOURCES` or `QUIET` mean in my command." | Use `commands:` — teach it the argument structure. |
| "The formatter knows the command fine, but I want it wider / different casing." | Use `per_command_overrides:` — change the layout knobs only. |

In other words: `commands:` is about _what_ the arguments mean; `per_command_overrides:` is about _how_ they get laid out on the page.

## Per-command Overrides

Use `per_command_overrides:` to change formatting knobs for one command name
without touching that command's argument syntax.

Example:

```yaml
per_command_overrides:
  my_custom_command:
    line_width: 120
    command_case: unchanged
    keyword_case: upper
    tab_size: 4
    dangle_parens: false
    dangle_align: prefix
    max_hanging_wrap_positional_args: 8
    max_hanging_wrap_groups: 3
```

Supported override fields:

- `command_case`
- `keyword_case`
- `line_width`
- `tab_size`
- `dangle_parens`
- `dangle_align`
- `max_hanging_wrap_positional_args`
- `max_hanging_wrap_groups`

Use this when you want a command to format differently from the global defaults.
Do **not** use it to define a command's argument structure — that belongs in
`commands:`.

## Custom Command Specs

Use `commands:` to teach `cmakefmt` about custom functions and macros, or to
override the built-in shape of an existing command.

Without a spec, `cmakefmt` sees every token as an undifferentiated positional
argument and has no way to group keyword sections or recognize flags. With a
spec, it understands the structure and can produce properly grouped, readable
output.

### Before and after

Consider this call — long enough to exceed the default `line_width` of 80:

```text
my_library(mylib SOURCES src/foo.cpp src/bar.cpp src/baz.cpp LIBRARIES Boost::filesystem fmt::fmt spdlog::spdlog QUIET)
```

**Without a spec**, `cmakefmt` has no idea that `SOURCES` and `LIBRARIES` are
keywords or that `QUIET` is a flag. It treats every token as a positional
argument and wraps them all at the same indent level, with no grouping:

```text
my_library(
  mylib
  SOURCES
  src/foo.cpp
  src/bar.cpp
  src/baz.cpp
  LIBRARIES
  Boost::filesystem
  fmt::fmt
  spdlog::spdlog
  QUIET)
```

After adding this spec:

```yaml
commands:
  my_library:
    pargs: 1
    flags:
      - QUIET
    kwargs:
      SOURCES:
        nargs: "+"
      LIBRARIES:
        nargs: "+"
```

**With a spec**, `cmakefmt` groups each keyword with its values in a
hanging-wrap layout, and recognizes `QUIET` as a standalone flag:

```text
my_library(
  mylib
  SOURCES src/foo.cpp src/bar.cpp src/baz.cpp
  LIBRARIES Boost::filesystem fmt::fmt spdlog::spdlog
  QUIET)
```

### Spec fields

#### `pargs`

How many positional arguments appear before keyword/flag processing begins.
Accepts an integer or one of the `nargs` strings described below.

```yaml
commands:
  my_command:
    pargs: 2      # exactly two positional args before keywords
```

#### `flags`

A list of standalone keyword tokens that take no values. When `cmakefmt` sees
one of these tokens it treats it as a boolean flag, not the start of a keyword
section.

```yaml
commands:
  my_command:
    flags:
      - QUIET
      - REQUIRED
      - GLOBAL
```

#### `kwargs`

Keyword sections that accept one or more values. Each entry names the keyword
and specifies how many values follow it via `nargs`.

```yaml
commands:
  my_command:
    kwargs:
      OUTPUT_DIRECTORY:
        nargs: 1        # exactly one value
      SOURCES:
        nargs: "+"      # one or more values
      OPTIONAL_SOURCES:
        nargs: "*"      # zero or more values
      MAYBE_ONE:
        nargs: "?"      # zero or one value
      AT_LEAST_TWO:
        nargs: "2+"     # two or more values
```

**`nargs` values:**

| Value | Meaning |
| --- | --- |
| `1`, `2`, … | Exactly that many values |
| `"*"` | Zero or more values |
| `"+"` | One or more values |
| `"?"` | Zero or one value |
| `"N+"` e.g. `"2+"` | At least N values |

Keywords can also nest their own `flags` and `kwargs` for commands with
multi-level structure:

```yaml
commands:
  my_command:
    kwargs:
      INSTALL:
        nargs: 0
        kwargs:
          DESTINATION:
            nargs: 1
        flags:
          - OPTIONAL
```

#### `layout`

Per-command layout hints that override the global config for this command only.

```yaml
commands:
  my_command:
    pargs: 1
    kwargs:
      SOURCES:
        nargs: "+"
    layout:
      always_wrap: true      # always use vertical layout
      line_width: 120        # allow wider lines for this command
      tab_size: 4
      dangle_parens: true
```

### Discriminated commands

Some commands dispatch on their first token — `file()`, `install()`, and
`string()` are typical examples. You can define a separate form for each
discriminator and an optional fallback:

```yaml
commands:
  my_command:
    forms:
      READ:
        pargs: 1
        kwargs:
          INTO:
            nargs: 1
      WRITE:
        pargs: 1
        kwargs:
          CONTENT:
            nargs: 1
    fallback:
      pargs: "*"
```

`cmakefmt` picks the form whose key matches the first argument
(case-insensitively) and falls back to `fallback` when nothing matches.

### A larger example

```yaml
commands:
  my_library:
    pargs: 1
    flags:
      - QUIET
      - EXCLUDE_FROM_ALL
    kwargs:
      SOURCES:
        nargs: "+"
      HEADERS:
        nargs: "*"
      LIBRARIES:
        nargs: "+"
      INSTALL:
        nargs: 0
        kwargs:
          DESTINATION:
            nargs: 1
        flags:
          - OPTIONAL
    layout:
      always_wrap: true
```

With this spec, given:

```text
my_library(mylib SOURCES src/foo.cpp src/bar.cpp src/baz.cpp HEADERS include/foo.h include/bar.h LIBRARIES Boost::filesystem fmt::fmt INSTALL DESTINATION lib/mylib OPTIONAL QUIET)
```

`cmakefmt` produces:

```text
my_library(
  mylib
  SOURCES src/foo.cpp src/bar.cpp src/baz.cpp
  HEADERS include/foo.h include/bar.h
  LIBRARIES Boost::filesystem fmt::fmt
  INSTALL DESTINATION lib/mylib OPTIONAL
  QUIET)
```

`always_wrap: true` in the layout forces the vertical layout unconditionally.
Each keyword group starts on its own line. Without this flag, short calls that
fit within `line_width` would stay on a single line.

For larger custom specs, YAML requires less punctuation and is easier to read
with deeply nested structures, which is why the default starter config is YAML.

## Old Draft Key Names

The current `cmakefmt` config schema only accepts the clearer names on this
page. If you have an older local config, rename any of the following before use:

| Old key | New key |
| --- | --- |
| `use_tabchars` | `use_tabs` |
| `max_pargs_hwrap` | `max_hanging_wrap_positional_args` |
| `max_subgroups_hwrap` | `max_hanging_wrap_groups` |
| `separate_ctrl_name_with_space` | `space_before_control_paren` |
| `separate_fn_name_with_space` | `space_before_definition_paren` |

`cmakefmt` fails fast on unknown config keys rather than silently ignoring them
— so you will know immediately if any remain.

## Related Reading

- [CLI Reference](/cli/)
- [Formatter Behavior](/behavior/)
- [Troubleshooting](/troubleshooting/)
