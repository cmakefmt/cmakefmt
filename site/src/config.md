# Config Reference

## Config Discovery Order

1. repeated `--config-file <PATH>` files, if provided
2. the nearest `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` found by walking upward from the file
3. `~/.cmakefmt.yaml`, `~/.cmakefmt.yml`, or `~/.cmakefmt.toml`
4. built-in defaults

If multiple supported config filenames exist in the same directory, YAML is preferred over TOML.

YAML is the recommended user-facing format once your config includes larger custom-command specs. `--dump-config` emits YAML by default, and `--dump-config toml` prints the TOML variant.

## Defaults

```toml
[format]
line_width = 80
tab_size = 2
use_tabs = false
max_empty_lines = 1
max_hanging_wrap_lines = 2
max_hanging_wrap_positional_args = 6
max_hanging_wrap_groups = 2
dangle_parens = false
dangle_align = "prefix"
min_prefix_length = 4
max_prefix_length = 10
space_before_control_paren = false
space_before_definition_paren = false

[style]
command_case = "lower"
keyword_case = "upper"

[markup]
enable_markup = true
reflow_comments = false
first_comment_is_literal = true
literal_comment_pattern = ""
bullet_char = "*"
enum_char = "."
fence_pattern = "^\s*[`~]{3}[^`\n]*$"
ruler_pattern = "^[^\w\s]{3}.*[^\w\s]{3}$"
hashruler_min_length = 10
canonicalize_hashrulers = true
```

## Format Options

| Option | Meaning |
| --- | --- |
| `line_width` | Target maximum output width. |
| `tab_size` | Spaces per indent level when not using tabs. |
| `use_tabs` | Use tabs for leading indentation. |
| `max_empty_lines` | Maximum blank-line runs preserved by formatting. |
| `max_hanging_wrap_lines` | Maximum number of hanging-wrap lines before vertical fallback. |
| `max_hanging_wrap_positional_args` | Maximum positional args before vertical fallback. |
| `max_hanging_wrap_groups` | Maximum subgroup packing before vertical fallback. |
| `dangle_parens` | Place closing paren on a separate line when wrapping. |
| `dangle_align` | Align dangling paren to `prefix`, `open`, or `close`. |
| `min_prefix_length`, `max_prefix_length` | Prefix heuristics used by layout decisions. |
| `space_before_control_paren` | Insert a space before control-flow parentheses. |
| `space_before_definition_paren` | Insert a space before function/macro definition parentheses. |

## Style Options

| Option | Meaning |
| --- | --- |
| `command_case` | `lower`, `upper`, or `unchanged`. |
| `keyword_case` | `lower`, `upper`, or `unchanged`. |

## Markup Options

| Option | Meaning |
| --- | --- |
| `enable_markup` | Enable comment-markup awareness. |
| `reflow_comments` | Reflow line comments to fit within the configured width. |
| `first_comment_is_literal` | Treat the first comment block literally. |
| `literal_comment_pattern` | Regex for comments that should not be rewritten. |
| `bullet_char`, `enum_char` | Preferred bullet/enum markers when markup normalization is enabled. |
| `fence_pattern` | Fence regex for literal comment regions. |
| `ruler_pattern`, `hashruler_min_length`, `canonicalize_hashrulers` | Hash-ruler detection and normalization controls. |

## Per-command Overrides

The `[per_command.<name>]` table supports these overrides:

- `command_case`
- `keyword_case`
- `line_width`
- `tab_size`
- `dangle_parens`
- `dangle_align`
- `max_hanging_wrap_positional_args`
- `max_hanging_wrap_groups`

Example:

```yaml
per_command:
  message:
    line_width: 120
    command_case: unchanged
    keyword_case: upper
```

These tables only change formatting knobs for a command name. They do not
define command syntax.

## Custom Command Specs

Use `commands.<name>` to teach `cmakefmt` about custom functions/macros or
to override the built-in shape of an existing command.

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

For user config, prefer YAML for custom commands beyond the smallest flat
cases. TOML remains supported, but YAML is easier to scan and maintain for
larger nested specs.

The same command-spec format is used by the built-in registry in
`src/spec/builtins.toml`.

The current `cmakefmt` config schema only accepts the clearer names on this
page. If you have an older local config draft using keys like
`use_tabchars`, `max_pargs_hwrap`, or `separate_ctrl_name_with_space`, update
it to the new spellings before using it.
