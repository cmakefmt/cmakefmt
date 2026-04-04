# Config Reference

This page explains the full user-facing config schema for `cmakefmt`.

The short version:

- user config may be YAML or TOML
- YAML is the recommended default for hand-edited configs
- custom command syntax goes under `commands:`
- command-specific layout/style tweaks go under `per_command_overrides:`

## Config Discovery Order

For a given target file, `cmakefmt` resolves config in this order:

1. repeated `--config-file <PATH>` files, if provided
2. the nearest `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` found by walking upward from the target
3. `~/.cmakefmt.yaml`, `~/.cmakefmt.yml`, or `~/.cmakefmt.toml`
4. built-in defaults

If multiple supported config filenames exist in the same directory, YAML is
preferred over TOML.

Use these commands when you want to inspect what happened:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --explain-config src/CMakeLists.txt
```

## Recommended Starter File

YAML is the recommended user-facing format:

```yaml
format:
  line_width: 80
  tab_size: 2

style:
  command_case: lower
  keyword_case: upper
```

Generate the full starter template with:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

If you want TOML instead:

```bash
cmakefmt --dump-config toml > .cmakefmt.toml
```

## Table Of Contents

- [Format Options](#format-options)
  - [`line_width`](#line_width)
  - [`tab_size`](#tab_size)
  - [`use_tabs`](#use_tabs)
  - [`max_empty_lines`](#max_empty_lines)
  - [`max_hanging_wrap_lines`](#max_hanging_wrap_lines)
  - [`max_hanging_wrap_positional_args`](#max_hanging_wrap_positional_args)
  - [`max_hanging_wrap_groups`](#max_hanging_wrap_groups)
  - [`dangle_parens`](#dangle_parens)
  - [`dangle_align`](#dangle_align)
  - [`min_prefix_length`](#min_prefix_length)
  - [`max_prefix_length`](#max_prefix_length)
  - [`space_before_control_paren`](#space_before_control_paren)
  - [`space_before_definition_paren`](#space_before_definition_paren)
- [Style Options](#style-options)
  - [`command_case`](#command_case)
  - [`keyword_case`](#keyword_case)
- [Markup Options](#markup-options)
  - [`enable_markup`](#enable_markup)
  - [`reflow_comments`](#reflow_comments)
  - [`first_comment_is_literal`](#first_comment_is_literal)
  - [`literal_comment_pattern`](#literal_comment_pattern)
  - [`bullet_char`](#bullet_char)
  - [`enum_char`](#enum_char)
  - [`fence_pattern`](#fence_pattern)
  - [`ruler_pattern`](#ruler_pattern)
  - [`hashruler_min_length`](#hashruler_min_length)
  - [`canonicalize_hashrulers`](#canonicalize_hashrulers)
- [Per-command Overrides](#per-command-overrides)
- [Custom Command Specs](#custom-command-specs)
- [Old Draft Key Names](#old-draft-key-names)

## Defaults

```yaml
format:
  line_width: 80
  tab_size: 2
  use_tabs: false
  max_empty_lines: 1
  max_hanging_wrap_lines: 2
  max_hanging_wrap_positional_args: 6
  max_hanging_wrap_groups: 2
  dangle_parens: false
  dangle_align: prefix
  min_prefix_length: 4
  max_prefix_length: 10
  space_before_control_paren: false
  space_before_definition_paren: false

style:
  command_case: lower
  keyword_case: upper

markup:
  enable_markup: true
  reflow_comments: false
  first_comment_is_literal: true
  literal_comment_pattern: ""
  bullet_char: "*"
  enum_char: "."
  fence_pattern: "^\\s*[`~]{3}[^`\\n]*$"
  ruler_pattern: "^[^\\w\\s]{3}.*[^\\w\\s]{3}$"
  hashruler_min_length: 10
  canonicalize_hashrulers: true
```

## Format Options

### `line_width`

Target maximum output width before wrapping is attempted.

```yaml
format:
  line_width: 100
```

Use a larger value if your project prefers wider CMake calls.

### `tab_size`

Indent width, in spaces, when `use_tabs` is `false`.

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

This affects leading indentation only. It does not change internal alignment
rules beyond the indentation unit.

### `max_empty_lines`

Maximum number of consecutive blank lines preserved by formatting.

```yaml
format:
  max_empty_lines: 1
```

If an input file contains larger blank-line runs, `cmakefmt` will clamp them
down to this limit.

### `max_hanging_wrap_lines`

Maximum number of lines that a hanging-wrap layout is allowed to consume before
the formatter falls back to a more vertical layout.

```yaml
format:
  max_hanging_wrap_lines: 2
```

Use a smaller value to force more aggressively vertical layouts.

### `max_hanging_wrap_positional_args`

Maximum positional arguments to keep in a hanging-wrap layout before falling
back to a more vertical layout.

```yaml
format:
  max_hanging_wrap_positional_args: 6
```

This is especially noticeable on commands with long source/header lists.

### `max_hanging_wrap_groups`

Maximum number of keyword/flag subgroups to keep in a hanging-wrap layout.

```yaml
format:
  max_hanging_wrap_groups: 2
```

If a command becomes keyword-heavy, lowering this value pushes it toward a more
vertical layout earlier.

### `dangle_parens`

Place the closing `)` on its own line when a call wraps.

```yaml
format:
  dangle_parens: true
```

Example shape:

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

Most users should leave this alone unless they are tuning layout behavior very
deliberately.

### `max_prefix_length`

Upper heuristic bound used when deciding between compact and wrapped layouts.

```yaml
format:
  max_prefix_length: 10
```

Like `min_prefix_length`, this is mainly a layout-tuning knob rather than a
day-one config option.

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

## Style Options

### `command_case`

Controls casing of command names.

Allowed values:

- `lower`
- `upper`
- `unchanged`

```yaml
style:
  command_case: lower
```

### `keyword_case`

Controls casing of recognized keywords and flags.

Allowed values:

- `lower`
- `upper`
- `unchanged`

```yaml
style:
  keyword_case: upper
```

Example:

```cmake
target_link_libraries(foo PUBLIC bar)
```

with:

```yaml
style:
  command_case: lower
  keyword_case: upper
```

stays:

```cmake
target_link_libraries(foo PUBLIC bar)
```

and with:

```yaml
style:
  command_case: upper
  keyword_case: lower
```

becomes:

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

This allows the formatter to treat some comments as lists, fences, and rulers
instead of opaque text.

### `reflow_comments`

Reflow plain line comments to fit within the configured line width.

```yaml
markup:
  reflow_comments: true
```

If you want comments preserved more literally, leave this `false`.

### `first_comment_is_literal`

Preserve the first comment block in a file literally.

```yaml
markup:
  first_comment_is_literal: true
```

This is often useful for license headers or hand-crafted introductory comments.

### `literal_comment_pattern`

Regex for comments that should never be reflowed.

```yaml
markup:
  literal_comment_pattern: "^\\s*NOTE:"
```

Use this for project-specific comment conventions that should stay untouched.

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

Most users should keep the default unless they have a strong house style.

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

## Per-command Overrides

Use `per_command_overrides:` to change formatting knobs for one command name
without changing that command's syntax.

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

Use this when you want a command to format differently. Do **not** use it to
define a command's arguments.

## Custom Command Specs

Use `commands:` to teach `cmakefmt` about custom functions/macros or to
override the built-in shape of an existing command.

Example:

```yaml
commands:
  my_custom_command:
    pargs: 1
    flags:
      - QUIET
    kwargs:
      SOURCES:
        nargs: "+"
      LIBRARIES:
        nargs: "+"
```

This tells `cmakefmt` that:

- the command starts with one positional argument
- `QUIET` is a standalone flag
- `SOURCES` starts a keyword section with one or more values
- `LIBRARIES` starts a keyword section with one or more values

For larger custom specs, YAML is much easier to maintain than TOML. That is why
the default starter config is YAML.

## Old Draft Key Names

The current `cmakefmt` config schema only accepts the clearer names on this
page. Older local draft names such as:

- `use_tabchars`
- `max_pargs_hwrap`
- `max_subgroups_hwrap`
- `separate_ctrl_name_with_space`
- `separate_fn_name_with_space`

should be updated before use.

## Related Reading

- [CLI Reference](cli.md)
- [Formatter Behavior](behavior.md)
- [Troubleshooting](troubleshooting.md)
