# Architecture

## Pipeline

```
Source (String)
    │
    ▼  [pest grammar — src/parser/cmake.pest]
Concrete Syntax Tree (pest::iterators::Pairs)
    │
    ▼  [AST builder — src/parser/ast.rs]
Abstract Syntax Tree (Vec<Node>)
    │
    ▼  [Formatter — src/formatter/]
Doc IR  (pretty::Doc)
    │
    ▼  [Wadler-Lindig layout engine — `pretty` crate]
Formatted String
```

Comments are parsed as first-class tokens in the pest grammar and preserved
as `Node::LineComment` / `Node::BracketComment` throughout the pipeline.

---

## AST

The AST is a flat-ish tree. CMake has no nested expression grammar so there
is no deep recursion. The top-level structure is a `File` containing a sequence
of `Statement`s.

```rust
pub struct File {
    pub statements: Vec<Statement>,
}

pub enum Statement {
    Command(CommandInvocation),
    LineComment(LineComment),
    BracketComment(BracketComment),
    Whitespace(Whitespace),   // blank lines — preserved for spacing
}

pub struct CommandInvocation {
    pub name: Identifier,
    pub arguments: Vec<Argument>,
    pub leading_comments: Vec<Comment>,   // comments before the command
    pub trailing_comment: Option<Comment>, // comment on same line as closing paren
    pub span: Span,
}

pub enum Argument {
    Bracket(BracketArgument),
    Quoted(QuotedArgument),
    Unquoted(UnquotedArgument),
    // Separator (whitespace between args) is NOT an argument;
    // it's handled by the formatter.
}
```

Spans (byte offsets into the source) are preserved on all nodes so that
error messages can point at the right location.

---

## Command Spec Registry

The formatter needs to understand a command's argument *structure* to produce
intelligent output — specifically: which tokens are keywords that start a new
visual group, and how many arguments each keyword consumes.

Without this, `target_link_libraries` would be formatted as a flat token list
rather than visually grouped by `PUBLIC`/`PRIVATE`/`INTERFACE`.

### Core types (`src/spec/mod.rs`)

```rust
/// How many arguments a position accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NArgs {
    Fixed(usize),           // exactly N:        "pargs = 1"
    ZeroOrMore,             // zero or more:     "pargs = \"*\""
    OneOrMore,              // one or more:      "pargs = \"+\""
    Optional,               // zero or one:      "pargs = \"?\""
    AtLeast(usize, char),   // N or more:        "pargs = \"2+\""  (parsed from "N+")
    Range(usize, usize),    // between N and M:  "pargs = [2, 4]"
}

/// Positional argument group spec (args before the first keyword).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosSpec {
    pub nargs: NArgs,
    /// If true the formatter may sort args alphabetically (e.g. source lists).
    #[serde(default)]
    pub sortable: bool,
}

/// Specification for a keyword's following arguments, and any nested
/// sub-keywords that keyword itself introduces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KwargSpec {
    pub nargs: NArgs,
    /// Sub-keywords introduced by this keyword (e.g. TARGETS in install()
    /// introduces DESTINATION, PERMISSIONS, etc.)
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Boolean flags valid inside this keyword's argument group.
    #[serde(default)]
    pub flags: IndexSet<String>,
}

/// The complete shape of one command invocation form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandForm {
    /// Positional args before the first keyword.  Default: ZeroOrMore.
    #[serde(default)]
    pub pargs: PosSpec,
    /// Keyword → spec for its argument group.
    #[serde(default)]
    pub kwargs: IndexMap<String, KwargSpec>,
    /// Boolean keywords (flags) that take no arguments.
    #[serde(default)]
    pub flags: IndexSet<String>,
    /// Per-command layout overrides (overrides global config for this command).
    #[serde(default)]
    pub layout: Option<LayoutOverrides>,
}

/// A command may have a single fixed shape, or multiple forms selected by the
/// value of the first positional argument (the "discriminator").
///
/// Example: install(TARGETS ...) vs install(FILES ...) vs install(DIRECTORY ...)
/// Each form has completely different keywords.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandSpec {
    /// All invocations have the same argument shape.
    Single(CommandForm),
    /// First positional argument selects the form.
    Discriminated {
        forms: IndexMap<String, CommandForm>,
        /// Form to use when the first arg doesn't match any key (or is absent).
        #[serde(default)]
        fallback: Option<CommandForm>,
    },
}
```

### Fallback for unknown commands

Any command **not** in the registry is formatted as if it has this spec:

```rust
CommandSpec::Single(CommandForm {
    pargs: PosSpec { nargs: NArgs::ZeroOrMore, sortable: false },
    kwargs: IndexMap::new(),
    flags: IndexSet::new(),
    layout: None,
})
```

i.e. all arguments are treated as positional tokens with no keyword grouping.
The Wadler-Lindig layout still applies — it just won't create per-keyword groups.

### Built-in registry

Shipped as an embedded TOML file (`src/spec/builtins.toml`, loaded via
`include_str!` at compile time). Covers all ~150 CMake built-in commands.

Example entries:

```toml
# ── Simple command ────────────────────────────────────────────────────────────
[commands.cmake_minimum_required]
pargs = "1"
flags = ["FATAL_ERROR"]

[commands.cmake_minimum_required.kwargs.VERSION]
nargs = "1"

# ── Keyword-heavy command ─────────────────────────────────────────────────────
[commands.target_link_libraries]
pargs = "1"   # the target name

[commands.target_link_libraries.kwargs.PUBLIC]
nargs = "*"

[commands.target_link_libraries.kwargs.PRIVATE]
nargs = "*"

[commands.target_link_libraries.kwargs.INTERFACE]
nargs = "*"

# ── Flag-heavy command ────────────────────────────────────────────────────────
[commands.find_package]
pargs = "1+"
flags = ["EXACT", "QUIET", "REQUIRED", "NO_POLICY_SCOPE", "MODULE", "CONFIG",
         "NO_MODULE", "GLOBAL", "OPTIONAL_COMPONENTS"]

[commands.find_package.kwargs.VERSION]
nargs = "1"

[commands.find_package.kwargs.COMPONENTS]
nargs = "+"

[commands.find_package.kwargs.OPTIONAL_COMPONENTS]
nargs = "+"

[commands.find_package.kwargs.HINTS]
nargs = "*"

[commands.find_package.kwargs.PATHS]
nargs = "*"

# ── Discriminated command (multiple forms) ────────────────────────────────────
[commands.install]
# install() dispatches on its first positional argument

[commands.install.forms.TARGETS]
pargs = "+"    # one or more targets
flags = ["OPTIONAL", "EXCLUDE_FROM_ALL", "NAMELINK_ONLY", "NAMELINK_SKIP"]

[commands.install.forms.TARGETS.kwargs.DESTINATION]
nargs = "1"

[commands.install.forms.TARGETS.kwargs.PERMISSIONS]
nargs = "+"

[commands.install.forms.TARGETS.kwargs.COMPONENT]
nargs = "1"

[commands.install.forms.FILES]
pargs = "+"
flags = ["OPTIONAL"]

[commands.install.forms.FILES.kwargs.DESTINATION]
nargs = "1"

[commands.install.forms.DIRECTORY]
pargs = "+"

[commands.install.forms.DIRECTORY.kwargs.DESTINATION]
nargs = "1"

# ── Tuple-valued kwargs (name-value pairs) ────────────────────────────────────
[commands.set_target_properties]
pargs = "+"    # one or more targets

[commands.set_target_properties.kwargs.PROPERTIES]
nargs = "*"    # treated as sequential name-value pairs by the formatter
```

### User extension via config

Users can add specs for their own macros/functions, or override built-ins, in
`.cmakefmt.toml`:

```toml
# Define your own cmake function's argument shape so the formatter groups it
[commands.my_add_executable]
pargs = "1"

[commands.my_add_executable.kwargs.SRCS]
nargs = "+"

[commands.my_add_executable.kwargs.DEPS]
nargs = "*"

[commands.my_add_executable.kwargs.INCLUDE_DIRS]
nargs = "*"

# Override a built-in's layout
[commands.message]
layout.always_wrap = true
```

User-defined entries are **merged** with the built-in registry; user entries win
on conflict.

### How the formatter uses a spec

Given `CommandSpec` for the current command, the formatter:

1. Classifies each argument token as: positional, keyword, flag, or unknown.
2. Groups arguments into *sections*: the leading positional group + one section
   per keyword occurrence.
3. Applies the Wadler-Lindig layout to each section independently:
   - Short sections stay on one line.
   - Long sections break vertically with the keyword on its own line.
4. Unknown tokens (not matching any keyword or flag) are folded into the nearest
   preceding section as additional positional args.

---

## Formatter / Doc IR

We use the `pretty` crate which implements the Wadler-Lindig algorithm.

Key Doc primitives we use:

| Primitive | Meaning |
|---|---|
| `text(s)` | Literal string, never broken |
| `line()` | A newline (or space when in flat/single-line mode) |
| `hardline()` | Unconditional newline |
| `group(doc)` | Try to lay out `doc` flat; break all `line()`s if it doesn't fit |
| `indent(doc)` | Increase indentation level inside `doc` |
| `concat([a, b])` | Concatenate two docs |
| `nil()` | Empty doc |

### Argument list layout

The core formatting decision for CMake is how to lay out argument lists.
The rule is simple and mirrors `cmake-format`'s behaviour:

```
if all arguments fit on the current line:
    command(ARG1 ARG2 ARG3)

else:
    command(
        ARG1
        ARG2
        ARG3
    )
```

This maps cleanly to a `group(...)` containing `line()` separators:

```rust
// Pseudo-code
let args_doc = group(
    indent(
        concat(args.iter().map(|a| concat([line(), format_arg(a)])))
    )
    + line()
);
let doc = text(name) + text("(") + args_doc + text(")");
```

When `group` fits on one line, `line()` renders as a space.
When it doesn't, every `line()` renders as a newline + indentation.

### Blank lines between statements

Blank lines between top-level commands are preserved up to a configurable
maximum (`max_empty_lines_between_commands`, default 1). This is handled by
examining the `Whitespace` nodes in the AST.

---

## Config

The config file is `.cmakefmt.toml`. Every option mirrors or improves on
the original `cmake-format` Python tool. All options have sane defaults so
the tool works out of the box with no config file.

```toml
# .cmakefmt.toml  —  full reference with defaults shown

# ── Layout ────────────────────────────────────────────────────────────────────

[format]
# Maximum line width before the formatter switches from horizontal to
# vertical argument layout.
line_width = 80

# Spaces per indent level.
tab_size = 2

# Use tab characters instead of spaces for indentation.
use_tabchars = false

# Maximum number of consecutive blank lines preserved between statements.
# Extra blank lines above this are collapsed. (0 = no blank lines allowed)
max_empty_lines = 1

# If the argument list of a command fits within this many lines when laid
# out horizontally, keep it horizontal. Otherwise go vertical.
max_lines_hwrap = 2

# If the number of positional (non-keyword) arguments exceeds this, skip
# horizontal wrapping and go straight to vertical layout.
max_pargs_hwrap = 6

# If the number of argument sub-groups (e.g. keyword blocks like
# TARGET_SOURCES PUBLIC ...) exceeds this, skip horizontal wrapping.
max_subgroups_hwrap = 2

# ── Parenthesis style ─────────────────────────────────────────────────────────

# Place the closing paren on its own line when the argument list wraps.
#
#   dangle_parens = false (default):
#     command(
#         ARG1
#         ARG2)
#
#   dangle_parens = true:
#     command(
#         ARG1
#         ARG2
#     )
dangle_parens = false

# When dangle_parens = true, how to align the closing paren:
#   "prefix"  — align with the start of the command name
#   "open"    — align with the opening paren column
#   "close"   — no extra indent (flush with current indent level)
dangle_align = "prefix"

# If the column of the opening paren is less than this many characters from
# the start of the line, allow horizontal wrapping. If it is more, fall back
# to vertical immediately (the line is already long enough).
min_prefix_chars = 4
max_prefix_chars = 10

# Add a space between flow-control keywords and their opening paren.
#   false: if(condition)
#   true:  if (condition)
separate_ctrl_name_with_space = false

# Add a space between function/macro names and their opening paren in
# definitions (function and macro commands only).
#   false: function(my_func ARG)
#   true:  function (my_func ARG)
separate_fn_name_with_space = false

# ── Casing ────────────────────────────────────────────────────────────────────

[style]
# Normalise the case of command names (the word before the opening paren).
#   "lower"     — cmake_minimum_required(...)
#   "upper"     — CMAKE_MINIMUM_REQUIRED(...)
#   "unchanged" — leave as-is
command_case = "lower"

# Normalise the case of keyword arguments (ALL_CAPS words that are part of
# a command's defined keyword set, e.g. PUBLIC, PRIVATE, REQUIRED).
#   "lower"     — public, private, required
#   "upper"     — PUBLIC, PRIVATE, REQUIRED
#   "unchanged" — leave as-is
keyword_case = "upper"

# ── Comment markup ────────────────────────────────────────────────────────────

[markup]
# Enable reflow and markup processing for line comments.
# When true, adjacent line comments are treated as a paragraph and reflowed
# to fit within line_width. Set false to preserve all comments verbatim.
enable_markup = true

# When enable_markup = true, the first comment block at the top of the file
# (copyright/license headers) is excluded from reflowing.
first_comment_is_literal = true

# A regex pattern. Comment lines matching this pattern are passed through
# verbatim (no reflow, no markup processing).
# Example: "^# ====" to protect ruler lines you manage yourself.
literal_comment_pattern = ""

# Bullet character for unordered list items in reflowed comments.
bullet_char = "*"

# Delimiter character for ordered list items in reflowed comments.
enum_char = "."

# Regex identifying fenced code blocks in comments (like Markdown).
# Lines matching this pattern toggle verbatim mode — content inside is
# not reflowed.
fence_pattern = "^\\s*[`~]{3}[^`\\n]*$"

# Regex identifying horizontal ruler lines in comments.
# Rulers are preserved as-is (not reflowed).
ruler_pattern = "^[^\\w\\s]{3}.*[^\\w\\s]{3}$"

# Minimum width for a line of repeated hash characters to be treated as a
# "hash ruler" (preserved verbatim).
hashruler_min_length = 10

# If a hash ruler is found, normalise it to exactly line_width characters.
canonicalize_hashrulers = true

# ── Per-command overrides ─────────────────────────────────────────────────────

# Per-command config allows overriding layout options for specific commands.
# Useful for commands like set() or message() that have very different
# argument shapes.
#
# [per_command.set]
# command_case = "upper"     # override: always uppercase SET(...)
# line_width = 120           # allow wider lines for set() calls
#
# [per_command.message]
# dangle_parens = true

# Command specs teach cmakefmt the syntax of custom commands or override
# built-in command shapes. In user config, prefer the condensed inline
# kwargs = { ... } form when the command is small and flat.
#
# [commands.my_custom_command]
# pargs = 1
# flags = ["QUIET"]
# kwargs = { SOURCES = { nargs = "+" } }
```

Config is loaded by `src/config/`. Resolution order (highest wins):

1. CLI flags (e.g. `--line-width 120`)
2. repeated `--config <PATH>` files, if provided (later files override earlier ones)
3. the nearest `.cmakefmt.toml` found by walking up from the file
4. `~/.cmakefmt.toml` (user global default)
5. Compiled-in defaults (the values shown above)

---

## Error handling

All errors implement `std::error::Error` via `thiserror`.

```
CmakeFormatError
    ├── ParseError(pest::error::Error<Rule>)
    ├── ConfigError(toml::de::Error)
    ├── IoError(std::io::Error)
    └── FormatterError(String)   // internal invariant violations
```

The binary (`main.rs`) converts errors to user-friendly messages with
source location info where available. Exit codes:

- `0` — success (formatted or already correct)
- `1` — check mode: file would be reformatted
- `2` — parse or config error

---

## Testing strategy

### Unit tests

Each module has `#[cfg(test)]` tests for its core logic.

### Snapshot tests (`insta`)

Every distinct formatting behaviour has a snapshot test. Snapshots are
committed to the repo and reviewed before merging. Located in
`tests/snapshots/`.

### Idempotency tests

`tests/idempotency.rs` runs every fixture file through the formatter twice
and asserts `format(format(x)) == format(x)`. This is a hard invariant.

### Round-trip tests (future)

Parse → unparse should reproduce the original source modulo whitespace.
This validates parser correctness independently of the formatter.

### Fixtures

`tests/fixtures/` contains:

- `basic/` — simple commands, argument types
- `comments/` — all comment positions and types
- `edge_cases/` — bracket args, deep nesting, very long lines, empty files
- `real_world/` — real `CMakeLists.txt` files from popular open-source
  projects (CMake itself, LLVM, Qt, OpenCV, etc.)

### Benchmarks

`benches/formatter.rs` uses `criterion`. Baseline target: format a 1000-line
`CMakeLists.txt` in under 10ms on a modern laptop.

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `pest` | 2.x | PEG parser runtime |
| `pest_derive` | 2.x | Grammar → Rust codegen (proc macro) |
| `pretty` | 0.12.x | Wadler-Lindig Doc IR + layout |
| `clap` | 4.x | CLI argument parsing (derive) |
| `serde` | 1.x | Config deserialization |
| `toml` | 0.8.x | TOML config file parsing |
| `thiserror` | 1.x | Error type derivation |
| `insta` | 1.x | Snapshot testing |
| `criterion` | 0.5.x | Benchmarking |
| `walkdir` | 2.x | Config file discovery (walk to root) |

No async runtime. No unsafe code. Single-threaded (parallelism across files
can be added later with `rayon`).
