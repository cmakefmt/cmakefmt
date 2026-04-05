# Architecture

This document is the implementation-facing companion to the user docs in
`site/src/`. It explains how `cmakefmt` is structured internally, which data
flows through the pipeline, and where to make changes safely.

## Design Goals

The architecture is optimized around a few explicit priorities:

1. **Semantic safety.** Formatting must preserve the meaning of a file.
2. **Idempotency.** `format(format(x)) == format(x)` must keep holding.
3. **Structured formatting.** CMake is parsed and classified before layout;
   this is not a regex-based rewriter.
4. **Configurable command awareness.** Built-in and user-defined command specs
   drive grouping and layout decisions.
5. **Actionable diagnostics.** Parse/config/formatter failures should explain
   what happened and where.
6. **Workflow-friendly speed.** The CLI should be fast enough for local loops,
   hooks, and CI.

## Top-Level Pipeline

At the highest level, a formatting run looks like this:

```text
input selection
  -> config resolution
  -> parse source into AST
  -> resolve command form from command registry
  -> convert AST into pretty::Doc fragments
  -> render final text
  -> emit stdout / diff / check result / in-place rewrite
```

The CLI does the discovery/reporting part. The library entry points begin at
`format_source` and work from in-memory source text downward.

## Module Layout

The current code is still a single crate. The important modules are:

- `src/main.rs`
  - CLI parsing, workflow modes, discovery, summaries, reporting
- `src/lib.rs`
  - public library surface and re-exports
- `src/config/`
  - runtime config types
  - YAML/TOML config loading and merging
  - config template rendering
  - legacy `cmake-format` config conversion
- `src/parser/`
  - `pest` grammar
  - AST definitions
  - source parsing and parse errors
- `src/spec/`
  - command-shape model
  - embedded built-in registry
  - override merging
- `src/formatter/`
  - AST to pretty-doc layout conversion
  - comment handling
  - barrier/disabled-region handling
- `src/files.rs`
  - recursive CMake file discovery and ignore integration
- `src/error.rs`
  - shared cross-layer error types

## Parsing Pipeline

### Grammar

The source of truth for syntax is `src/parser/cmake.pest`.

The grammar covers:

- command invocations
- quoted arguments
- unquoted arguments
- bracket arguments
- line comments
- bracket comments
- variable references
- generator expressions
- continuation lines
- top-level template placeholders used by `.cmake.in` files

### AST Construction

`src/parser/ast.rs` converts the raw `pest` pairs into a more convenient AST.

The AST is intentionally simple. CMake does not have a rich nested expression
language, so the tree is mostly:

- file
- statement list
- command invocation
- argument list
- comments / whitespace / placeholders

Spans and line/column context are preserved so later errors can point back to
the correct source region.

### Comments As First-Class Syntax

Comments are not stripped and reattached later.

That is a deliberate design choice because it avoids a large class of bugs
where comment placement is guessed heuristically after formatting. Instead,
comments remain explicit syntax elements throughout parsing and formatting.

## Config Model

### Runtime Config

`src/config/mod.rs` defines the resolved `Config` struct that the formatter
actually consumes at runtime.

This struct is:

- fully typed
- populated from defaults
- merged with any discovered or explicit config files
- finally overridden by CLI flags

### File Formats

User config may be:

- `.cmakefmt.yaml`
- `.cmakefmt.yml`
- `.cmakefmt.toml`

YAML is the recommended user-facing format because it is much easier to read
and maintain once custom command specs become non-trivial.

### Config Resolution

`src/config/file.rs` is responsible for:

- loading explicit config files
- discovering nearest project config
- falling back to home-directory config
- rendering starter templates
- rendering effective config for `--show-config`
- converting legacy `cmake-format` config files

The key design constraint here is that config resolution must be inspectable.
That is why the CLI exposes:

- `--show-config-path`
- `--show-config`
- `--explain-config`

## Command Spec Registry

The formatter only produces good output if it understands command structure.

Without a registry, CMake would look like a flat token stream and commands such
as `target_link_libraries`, `install`, or project-specific DSL commands would
wrap poorly.

### Core Idea

Each command has a `CommandSpec`, which describes:

- leading positional arguments
- standalone flags
- keyword sections and their arity
- discriminated forms when the first positional argument changes the meaning

### Registry Sources

The registry is built from:

- embedded built-ins in `src/spec/builtins.toml`
- optional user overrides from config under `commands:`

Built-ins are compiled in with `include_str!` and normalized once. User
overrides are merged on top, which allows projects to:

- teach `cmakefmt` about custom macros/functions
- override built-in command shapes when necessary

### Why This Matters

For example, `target_link_libraries(foo PUBLIC bar PRIVATE baz)` should not be
formatted like four unrelated positional tokens. The registry is what tells the
formatter that `PUBLIC` and `PRIVATE` open new sections.

## Formatter Pipeline

### Public Entry Points

The main library entry points are:

- `format_source`
- `format_source_with_debug`
- `format_source_with_registry`
- `format_source_with_registry_debug`

These functions parse the source, select or receive a registry, and format the
AST into a final string.

### Pretty-Printing Model

The formatter uses the `pretty` crate, which implements a Wadler-Lindig style
document model.

The important consequence is that layout decisions are expressed as:

- text fragments
- soft line breaks
- hard line breaks
- indentation
- grouping

So the formatter can ask "does this group still fit flat?" instead of manually
splitting strings line by line.

### Layout Decisions

Most formatting behavior in `src/formatter/node.rs` is driven by:

- the effective `Config`
- the effective per-command override for the current command
- the selected `CommandSpec` form
- actual rendered width

Typical decisions include:

- keep on one line
- hanging wrap
- fully vertical fallback
- preserve disabled regions verbatim

### Comment Handling

`src/formatter/comment.rs` turns parsed comments into layout nodes while
preserving relative attachment:

- standalone comments
- inline/trailing comments
- bracket comments
- markup-aware comment handling when enabled

## CLI Workflow Layer

The CLI in `src/main.rs` is intentionally richer than "read file, write file".

It handles:

- recursive discovery
- `.cmakefmtignore` and `.gitignore`
- Git-aware modes such as `--staged` and `--changed`
- stdin formatting with `--stdin-path`
- range formatting with `--lines`
- `--check`, `--list-files`, `--diff`, and JSON reports
- progress bars and opt-in parallelism
- human summaries and batch error handling

This workflow layer is part of the product, not just a thin wrapper.

## Diagnostics Architecture

The project explicitly invests in better diagnostics.

That is why parse/config/spec errors preserve:

- file paths
- line and column information
- source snippets
- likely-cause hints where possible

And why the formatter exposes debug data about:

- file discovery
- config provenance
- barrier transitions
- chosen command forms
- chosen layout families

## Performance Model

The broad performance shape today is:

- parsing dominates end-to-end cost
- formatter layout is materially smaller than parser cost
- registry lookup is no longer a primary hotspot
- CLI overhead matters enough to optimize, especially on multi-file runs

The benchmark and profiling policy lives in `docs/PERFORMANCE.md`.

Practical consequence: if performance regresses, the most likely hotspots are:

- parser work
- unnecessary allocation/cloning
- repeated string normalization
- repeated CLI bookkeeping

## Invariants To Protect

When changing the parser, registry, formatter, or CLI, keep these invariants in
mind:

- formatted output should parse again
- parse-tree equivalence should hold modulo intentional whitespace normalization
- `format(format(x)) == format(x)` should hold
- comments and disabled regions must not be lost
- config discovery and diagnostics should remain explainable

If a change threatens one of those invariants, it is usually architectural, not
just cosmetic.

## Where To Extend The System

### Add New CMake Syntax

- grammar: `src/parser/cmake.pest`
- AST conversion: `src/parser/ast.rs`
- parser tests and fixtures

### Add New Command Knowledge

- built-ins: `src/spec/builtins.toml`
- merge/lookup logic: `src/spec/registry.rs`
- snapshot coverage for affected commands

### Add New Formatting Behavior

- layout logic: `src/formatter/node.rs`
- comment-specific behavior: `src/formatter/comment.rs`
- snapshot tests plus idempotency coverage

### Add New Config Knobs

- runtime config: `src/config/mod.rs`
- file schema/template rendering: `src/config/file.rs`
- CLI overrides if appropriate: `src/main.rs`
- docs and tests

## Related Docs

- `docs/cmake-grammar.md`
- `docs/PERFORMANCE.md`
- `docs/ROADMAP.md`
- `site/src/architecture.md`

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
use_tabs = false

# Maximum number of consecutive blank lines preserved between statements.
# Extra blank lines above this are collapsed. (0 = no blank lines allowed)
max_empty_lines = 1

# If the argument list of a command fits within this many lines when laid
# out horizontally, keep it horizontal. Otherwise go vertical.
max_hanging_wrap_lines = 2

# If the number of positional (non-keyword) arguments exceeds this, skip
# horizontal wrapping and go straight to vertical layout.
max_hanging_wrap_positional_args = 6

# If the number of argument sub-groups (e.g. keyword blocks like
# TARGET_SOURCES PUBLIC ...) exceeds this, skip horizontal wrapping.
max_hanging_wrap_groups = 2

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
min_prefix_length = 4
max_prefix_length = 10

# Add a space between flow-control keywords and their opening paren.
#   false: if(condition)
#   true:  if (condition)
space_before_control_paren = false

# Add a space between function/macro names and their opening paren in
# definitions (function and macro commands only).
#   false: function(my_func ARG)
#   true:  function (my_func ARG)
space_before_definition_paren = false

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
# [per_command_overrides.set]
# command_case = "upper"     # override: always uppercase SET(...)
# line_width = 120           # allow wider lines for set() calls
#
# [per_command_overrides.my_custom_command]
# dangle_parens = true

# Command specs teach cmakefmt the syntax of custom commands or override
# built-in command shapes. In user-facing config, prefer YAML once custom
# commands grow beyond the smallest flat cases. TOML remains supported and
# can be emitted explicitly with --dump-config toml.
#
# [commands.my_custom_command]
# pargs = 1
# flags = ["QUIET"]
# kwargs = { SOURCES = { nargs = "+" } }
```

Config is loaded by `src/config/`. Resolution order (highest wins):

1. CLI flags (e.g. `--line-width 120`)
2. repeated `--config-file <PATH>` files, if provided (later files override earlier ones)
3. the nearest `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` found by walking up from the file
4. `~/.cmakefmt.yaml`, `~/.cmakefmt.yml`, or `~/.cmakefmt.toml` (user global default)
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

Every distinct formatting behaviour has a snapshot test in
`tests/snapshots.rs`. Most expectations are inline `insta` snapshots and are
reviewed in normal diffs before merging.

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
- `real_world/` — manifest and helper files for a fetched corpus of real
  `CMakeLists.txt` files from popular open-source projects (CMake itself,
  LLVM, Qt, OpenCV, etc.)

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
