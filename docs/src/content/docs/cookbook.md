---
title: Formatting Cookbook
description: Common formatting goals and the config options that achieve them.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Don't know the option name? Start here. Each section shows a common
formatting goal, the config to achieve it, and a before/after example.

All examples assume the default config unless stated otherwise. Click the
config option name to jump to its full documentation in the
[Config Reference](/config/).

## I want to control line width

**Option:** [`line_width`](/config/#line_width) (default: `80`)

Narrower widths force more wrapping. Wider widths keep more on one line.

```yaml
format:
  line_width: 40
```

Before (`line_width: 80`):

```cmake
target_link_libraries(
  mylib
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog)
```

After (`line_width: 40`):

```cmake
target_link_libraries(
  mylib
  PUBLIC
    Boost::filesystem Boost::system
    fmt::fmt spdlog::spdlog)
```

## I want to change indentation width

**Option:** [`tab_size`](/config/#tab_size) (default: `2`)

Controls how many spaces each indentation level uses. Larger values
make nesting more visually distinct; smaller values save horizontal space.

```yaml
format:
  tab_size: 4
```

Before (`tab_size: 2`):

```cmake
if(ENABLE_TESTS)
  add_executable(test_runner test_main.cc)
endif()
```

After (`tab_size: 4`):

```cmake
if(ENABLE_TESTS)
    add_executable(test_runner test_main.cc)
endif()
```

## I want to use tabs instead of spaces

**Option:** [`use_tabchars`](/config/#use_tabchars) (default: `false`)

Some projects prefer tab characters for indentation so that each
developer's editor can display them at their preferred width.

```yaml
format:
  use_tabchars: true
```

Indentation uses tab characters instead of spaces. The visual width is
still controlled by `tab_size`.

## I want to change command name casing

**Option:** [`command_case`](/config/#command_case) (default: `lower`)

CMake is case-insensitive for command names, so `add_executable` and
`ADD_EXECUTABLE` are identical. This option enforces a consistent style
across your project.

```yaml
format:
  command_case: upper
```

Before:

```cmake
cmake_minimum_required(VERSION 3.20)
add_executable(myapp main.cc)
```

After:

```cmake
CMAKE_MINIMUM_REQUIRED(VERSION 3.20)
ADD_EXECUTABLE(myapp main.cc)
```

Set to `unchanged` to leave command names as they are in the source.

## I want to change keyword casing

**Option:** [`keyword_case`](/config/#keyword_case) (default: `upper`)

Keywords like `PUBLIC`, `PRIVATE`, `VERSION`, and `FATAL_ERROR` are also
case-insensitive in CMake. This option enforces consistent casing for
all recognized keywords and flags.

```yaml
format:
  keyword_case: lower
```

Before:

```cmake
target_link_libraries(mylib PUBLIC dep1 dep2)
cmake_minimum_required(VERSION 3.20 FATAL_ERROR)
```

After:

```cmake
target_link_libraries(mylib public dep1 dep2)
cmake_minimum_required(version 3.20 fatal_error)
```

## I want the closing parenthesis on its own line

**Option:** [`dangle_parens`](/config/#dangle_parens) (default: `false`)

Some teams prefer the closing `)` on its own line for wrapped commands,
similar to how many C/C++ styles place `}` on its own line. This makes
diffs cleaner when adding arguments at the end.

```yaml
format:
  dangle_parens: true
```

Before:

```cmake
target_link_libraries(
  mylib
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog)
```

After:

```cmake
target_link_libraries(
  mylib
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog
)
```

## I want a space before `(` in `if()` / `foreach()`

**Option:** [`separate_ctrl_name_with_space`](/config/#separate_ctrl_name_with_space) (default: `false`)

Some style guides prefer `if (CONDITION)` over `if(CONDITION)` to
visually distinguish control-flow statements from function calls.

```yaml
format:
  separate_ctrl_name_with_space: true
```

Before:

```cmake
if(ENABLE_TESTS)
  message(STATUS "running tests")
endif()
```

After:

```cmake
if (ENABLE_TESTS)
  message(STATUS "running tests")
endif ()
```

## I want to sort argument lists

**Options:** [`enable_sort`](/config/#enable_sort) and [`autosort`](/config/#autosort) (both default: `false`)

Dependency lists and source file lists are often added to over time and
end up in random order. Sorting them makes it easier to spot duplicates
and find entries at a glance.

```yaml
format:
  enable_sort: true
  autosort: true
```

Before:

```cmake
target_link_libraries(
  mylib
  PUBLIC
    spdlog::spdlog
    Boost::filesystem
    fmt::fmt
    Boost::system)
```

After:

```cmake
target_link_libraries(
  mylib
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog)
```

`autosort` sorts keyword sections where all arguments are simple unquoted
tokens (no variables or generator expressions). Sorting is
case-insensitive.

For finer control, mark specific sections as `sortable` in a custom
command spec — then only those sections are sorted, regardless of
`autosort`:

```yaml
format:
  enable_sort: true

commands:
  my_cmd:
    kwargs:
      SOURCES:
        nargs: "+"
        sortable: true
```

## I want to keep the variable name on the `set()` line

**Option:** [`wrap_after_first_arg`](/config/#wrap_after_first_arg) (default for `set()`: `true`)

This is already enabled for `set()` by default. The variable name stays
on the `set(` line even when the rest wraps:

```cmake
set(SOURCES
    main.cc utils.cc parser.cc formatter.cc config.cc)
```

To disable it:

```yaml
per_command_overrides:
  set:
    wrap_after_first_arg: false
```

Which produces:

```cmake
set(
  SOURCES
  main.cc
  utils.cc
  parser.cc)
```

## I want to limit blank lines

**Option:** [`max_empty_lines`](/config/#max_empty_lines) (default: `1`)

Over time, CMake files accumulate inconsistent blank-line spacing —
some sections have one blank line, others have three or four. This
option clamps consecutive blank lines to a consistent maximum.

```yaml
format:
  max_empty_lines: 0
```

Set to `0` to remove all blank lines between commands, `1` (the default)
for single spacing, or `2` to allow double spacing.

## I want strict line-width enforcement in CI

**Option:** [`require_valid_layout`](/config/#require_valid_layout) (default: `false`)

When enabled, the formatter returns an error if any formatted line
exceeds `line_width` — for example because a quoted string or bracket
argument is too long to break. This guarantees that your CI pipeline
catches overlong lines instead of silently accepting them.

```yaml
format:
  line_width: 40
  require_valid_layout: true
```

Example error output when a line can't fit within the configured width:

```text
error: line 2 is 74 characters wide, exceeding the limit of 40
hint: set line_width = 74 (or higher), add the command to always_wrap,
      or disable require_valid_layout
```

Useful in CI where you want a hard guarantee that no line exceeds
the limit.

## I want to force specific commands to always wrap

**Option:** [`always_wrap`](/config/#always_wrap) (default: `[]`)

Some commands are easier to read in vertical layout even when they'd
fit on one line — for example, `target_link_libraries` with a few short
dependencies. By default, the formatter keeps short calls inline. This
option forces named commands to always wrap vertically.

```yaml
format:
  always_wrap:
    - target_link_libraries
```

Before:

```cmake
target_link_libraries(mylib PUBLIC dep1 dep2)
```

After:

```cmake
target_link_libraries(
  mylib
  PUBLIC dep1 dep2)
```

## I want to override settings for one command

**Section:** [`per_command_overrides`](/config/#per_command_overrides)

Sometimes a single command looks better with different settings — for
example, `message()` strings are often long and read better with a wider
line width, while the rest of your project uses the default.

```yaml
per_command_overrides:
  message:
    line_width: 120
```

Before (global `line_width: 80`):

```cmake
message(
  STATUS
    "Building project ${PROJECT_NAME} version ${PROJECT_VERSION} for ${CMAKE_SYSTEM_NAME}")
```

After (with `message` overridden to `line_width: 120`):

```cmake
message(
  STATUS "Building project ${PROJECT_NAME} version ${PROJECT_VERSION} for ${CMAKE_SYSTEM_NAME}")
```

Any format option can be overridden per command name. The override
applies only to that command; all others use the global setting.

## I want to disable formatting for a section

Sometimes you have a block of CMake code with intentional non-standard
formatting — a hand-aligned table, a generated section, or a legacy
block you're not ready to touch. Wrap it in a disable region:

```cmake
# cmakefmt: off
set(SPECIAL   keep   this   exactly)
# cmakefmt: on
```

Also supported: `# cmake-format: off/on`, `# fmt: off/on`, `# ~~~`.

See [Disabled Regions](/behavior/#disabled-regions-and-fences) for details.

## I want to stop comment reflow

**Option:** [`enable_markup`](/config/#enable_markup) (default: `true`)

By default, `cmakefmt` reflows long comments to fit within `line_width`.
If your comments contain ASCII art, pre-formatted tables, license
headers, or other content that should not be rewrapped, disable this.

```yaml
markup:
  enable_markup: false
```

With `enable_markup: true` (default), a long comment is reflowed:

```cmake
# This is a long comment that explains why we need this particular
# dependency and what it does for the build system
set(FOO bar)
```

With `enable_markup: false`, the same comment is preserved as written:

```cmake
# This is a long comment that explains why we need this particular dependency and what it does for the build system
set(FOO bar)
```

## I want to teach cmakefmt my custom commands

**Section:** [`commands`](/config/#custom-commands)

Without a spec, custom commands format as flat token lists. With a spec,
keywords and flags are recognized and arguments are grouped properly:

```yaml
commands:
  my_add_test:
    kwargs:
      NAME:
        nargs: 1
      SOURCES:
        nargs: "+"
      LIBRARIES:
        nargs: "+"
```

See [My Custom Command Formats Poorly](/troubleshooting/#my-custom-command-formats-poorly)
for a full before/after example.

## I want to review formatting changes side-by-side in meld / vimdiff / kdiff3

`cmakefmt --diff` emits a unified diff to stdout, which is the right
format for terminal review and for piping into tools that consume
unified-diff input (`delta`, `diff-so-fancy`). Side-by-side tools such as
meld, vimdiff, and kdiff3 expect two file paths instead, and the formatter
does not launch them directly. Use shell process substitution (Bash or
Zsh) to feed `cmakefmt`'s formatted output to such tools as if it were a
file:

```bash
# Open meld with the original on the left and the formatted output on the right
meld CMakeLists.txt <(cmakefmt CMakeLists.txt)

# Same idea with vimdiff
vimdiff CMakeLists.txt <(cmakefmt CMakeLists.txt)

# Or kdiff3
kdiff3 CMakeLists.txt <(cmakefmt CMakeLists.txt)
```

To review every changed file in a directory in turn, combine
`--list-changed-files` with a shell loop:

```bash
for f in $(cmakefmt --list-changed-files .); do
  meld "$f" <(cmakefmt "$f")
done
```

Visual Studio Code users get a built-in side-by-side diff with no shell
dance:

```bash
code --diff CMakeLists.txt <(cmakefmt CMakeLists.txt)
```

Process substitution (`<(...)`) is a Bash and Zsh feature; POSIX `sh` and
the Windows `cmd` shell do not support it. On Windows, use Git Bash, WSL,
or PowerShell with a temporary file:

```powershell
$tmp = New-TemporaryFile
cmakefmt CMakeLists.txt | Set-Content $tmp
meld CMakeLists.txt $tmp
Remove-Item $tmp
```
