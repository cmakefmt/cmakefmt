# cmakefmt — Project Guide

`cmakefmt` — a Rust reimplementation of `cmake-format` (from the dead
`cmakelang` Python project). Goal: fast, correct, configurable CMake
formatter distributed as a single binary.

## Key decisions (do not revisit without good reason)

- **Parser**: hand-written recursive descent over a streaming scanner. Public
  AST stays in `src/parser/ast.rs`; parser internals live in
  `src/parser/{cursor,scanner,grammar,lower}.rs`.
- **Formatter**: Wadler-Lindig algorithm via the `pretty` crate. AST -> Doc IR -> string.
- **Config**: YAML/TOML via `serde`. Files: `.cmakefmt.yaml`, `.cmakefmt.yml`, `.cmakefmt.toml`.
- **CLI**: `clap` (derive API). Subcommands: `config`, `dump`, `lsp`, `completions`, `install-hook`.
- **Snapshot tests**: `insta` crate. Snapshots live in `tests/snapshots/`.
- **Idempotency invariant**: `format(format(x)) == format(x)` must always hold.
- **Semantic verification**: formatted output is re-parsed and compared to the original AST
  (with comments stripped) to ensure formatting never changes CMake semantics.

## Build & run

```bash
cargo build                        # debug build
cargo build --release              # release build
cargo run -- path/to/CMakeLists.txt
cargo run -- --check path/to/CMakeLists.txt
cargo run -- -i path/to/CMakeLists.txt     # in-place
cargo run -- dump ast path/to/CMakeLists.txt   # debug: show parser AST
cargo run -- dump parse path/to/CMakeLists.txt # debug: show spec-resolved tree
```

## Testing

```bash
cargo test                         # all tests
cargo test parser                  # parser unit tests
cargo test formatter               # formatter unit tests
cargo test --test snapshots        # snapshot tests only
cargo test --test cli              # CLI integration tests
cargo test --test real_world       # real-world corpus tests
cargo test --test idempotency      # idempotency invariant
cargo insta review                 # review snapshot changes interactively
```

Idempotency tests run automatically as part of `cargo test`.

## Linting & formatting

```bash
cargo clippy -- -D warnings        # must pass clean
cargo fmt --check                  # must pass clean
```

CI and pre-commit hooks run both of these.

## Project layout

```bash
cmakefmt/
+-- AGENTS.md                  <- this file
+-- Cargo.toml
+-- docs/
|   +-- README.md              <- docs site contributor notes
|   +-- astro.config.mjs       <- Astro + Starlight site configuration
|   +-- package.json           <- docs app dependencies and scripts
|   +-- public/                <- static site assets
|   +-- src/
|       +-- content/docs/      <- published docs pages
|       +-- assets/            <- docs site branding assets
|       +-- styles/            <- Starlight theme customizations
+-- src/
|   +-- main.rs                <- CLI entry point (clap derive API)
|   +-- lib.rs                 <- public API (format_source, format_file)
|   +-- error.rs               <- unified error type (thiserror)
|   +-- files.rs               <- CMake file discovery, .cmakefmtignore support
|   +-- dump.rs                <- parse-tree dump (dump ast, dump parse)
|   +-- config/
|   |   +-- mod.rs             <- Config struct, defaults, resolution logic
|   |   +-- file.rs            <- YAML/TOML config file loading + merging
|   |   +-- editorconfig.rs    <- .editorconfig fallback support
|   |   +-- legacy.rs          <- conversion from legacy cmake-format config files
|   +-- parser/
|   |   +-- mod.rs             <- public parse() fn, parser glue
|   |   +-- cursor.rs          <- byte cursor over source text
|   |   +-- scanner.rs         <- streaming scanner for literals/comments/args
|   |   +-- grammar.rs         <- recursive-descent parser to private parse tree
|   |   +-- lower.rs           <- parse-tree -> public AST normalization
|   |   +-- ast.rs             <- AST node types (File, Statement, Argument, Comment)
|   +-- spec/
|   |   +-- mod.rs             <- NArgs, PosSpec, KwargSpec, CommandForm, CommandSpec
|   |   +-- registry.rs        <- CommandRegistry: loads builtins + user overrides
|   |   +-- builtins.yaml      <- built-in specs for all ~150 CMake commands
|   +-- formatter/
|   |   +-- mod.rs             <- public format() fn, barrier/fence handling
|   |   +-- node.rs            <- AST -> Doc IR (sections, wrapping, sorting)
|   |   +-- comment.rs         <- comment formatting and reflow
|   +-- lsp.rs                 <- LSP server (textDocument/formatting, rangeFormatting)
+-- tests/
|   +-- snapshots/             <- insta snapshot files (committed to repo)
|   +-- cli.rs                 <- CLI integration tests (~190 tests)
|   +-- idempotency.rs         <- format(format(x)) == format(x) + line width checks
|   +-- parser_fixtures.rs     <- parser unit tests driven by fixture files
|   +-- real_world.rs          <- formatting tests against pinned real-world corpus
|   +-- snapshots.rs           <- snapshot-based formatting tests (~66 tests)
|   +-- fixtures/              <- .cmake input files for tests
|       +-- basic/
|       +-- comments/
|       +-- edge_cases/
|       +-- real_world/
+-- benches/
|   +-- formatter.rs           <- criterion benchmarks
+-- packaging/                 <- Homebrew, winget, Scoop, conda-forge manifests
+-- scripts/                   <- sync-changelog.py, check-docs.sh, etc.
+-- fuzz/                      <- cargo-fuzz targets
```

## Comment handling

Comments are first-class AST nodes — they are NOT stripped and reattached.
The parser preserves their position relative to surrounding nodes, and the
formatter treats them like any other node in the Doc IR.

Types of comments in CMake:

- Line comment: `# ...` (to end of line)
- Bracket comment: `#[=[ ... ]=]` (multi-line)

### Trailing comment reflow

Long trailing comments that exceed `line_width` are reflowed with
continuation lines aligned to the `#`. The parser merges column-aligned
continuation comments back into the trailing comment on re-parse, ensuring
idempotent round-trips. This is gated behind `enable_markup`.

### Comment continuation merging (parser)

When a standalone line comment immediately follows a command's trailing
comment and the `#` is at the same column, the parser merges it into the
trailing comment text. This ensures the formatter's reflowed output
round-trips correctly. See `merge_trailing_comment_continuation` in
`src/parser/mod.rs`.

## Config precedence (highest -> lowest)

1. CLI flags (`--line-width`, `--tab-size`, etc.)
2. `.cmakefmt.yaml` / `.cmakefmt.toml` in the directory of the file being formatted
3. Same files walking up to repo root (or git root)
4. `.editorconfig` fallback for `indent_style` and `indent_size`
5. Built-in defaults

Config sections: `format:`, `markup:`, `per_command_overrides:`, `commands:`.
The full schema is at `cmakefmt config schema`.

### Key config options

- `format.line_width` (default 80) — maximum line width
- `format.tab_size` (default 2) — spaces per indent level
- `format.command_case` / `format.keyword_case` — `lower`, `upper`, or `unchanged`
- `format.dangle_parens` — closing paren on own line
- `format.wrap_after_first_arg` — keep first arg on command line (default for `set()`)
- `format.enable_sort` / `format.autosort` — sort arguments in keyword sections
- `markup.enable_markup` — controls comment reflow and markup handling
- `commands:` — user-defined command specs (pargs, kwargs, flags, sortable)
- `per_command_overrides:` — per-command format option overrides

## CLI subcommands and flags

### Subcommands

- `cmakefmt config dump|schema|check|show|path|explain|convert|init`
- `cmakefmt dump ast|parse` — debug: print parser AST or spec-resolved tree
- `cmakefmt lsp` — LSP server on stdio
- `cmakefmt completions <shell>` — shell completion scripts
- `cmakefmt install-hook` — install git pre-commit hook

### Key flags

- `-i` / `--in-place` — rewrite files on disk
- `--check` — exit 1 if any file would change
- `--diff` — show unified diff
- `--verify` / `--no-verify` — semantic verification (on by default for `-i`)
- `--debug` — verbose diagnostics to stderr
- `--explain` — show why a specific command was formatted its way
- `--color auto|always|never` — ANSI color
- `--parallel [N]` — parallel formatting (default: CPUs - 1)
- `--staged` / `--changed` — VCS-aware file selection
- `--lines START:END` — range formatting
- `--summary` — per-file status lines
- `--report-format human|json|github|checkstyle|junit|sarif|edit`

## Semantic verification

The semantic verifier (`verify_semantics` in `src/main.rs`) parses both
original and formatted output, normalizes them, and compares:

- Strips all `Statement::Comment` and `Statement::BlankLines`
- Strips `trailing_comment` and `Argument::InlineComment` from commands
- Normalizes command names and keyword case
- Zeroes out spans

Comments are excluded because they have no CMake semantic meaning and
comment reflow changes their structure. The idempotency test in
`tests/idempotency.rs` uses the same normalization — keep them in sync.

## Spec registry

Built-in specs live in `src/spec/builtins.yaml`. Format:

```yaml
commands:
  set:
    pargs: '2+'
    kwargs:
      CACHE:
        nargs: 2
        flags: [FORCE]
    layout:
      wrap_after_first_arg: true
```

- `registry.get("command_name")` -> `&CommandSpec`
- `spec.form_for(first_arg)` -> `&CommandForm` (handles discriminated commands like `install`)
- `split_sections(command, form)` classifies arguments as keywords, flags, or positionals

## Sorting

- `enable_sort: true` sorts sections marked `sortable: true` in the spec
- `autosort: true` heuristically sorts sections where all non-comment
  arguments are simple unquoted tokens (no variables/generator expressions)
- Sort is case-insensitive and stable
- Inline comments do not prevent autosort from activating

## Coding conventions

- SPDX headers on all source files:

  ```rust
  // SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
  // SPDX-License-Identifier: MIT OR Apache-2.0
  ```

- All public types derive `Debug`, `Clone`.
- Errors use `thiserror`. Never use `.unwrap()` in library code; use `?`.
- Pest rule names: `snake_case`. AST enum variants: `PascalCase`.
- New formatter rules go in `formatter/node.rs` alongside the relevant AST variant.
- Every new formatting behaviour needs a snapshot test.
- Pre-commit hooks: `trim-trailing-whitespace`, `cargo fmt`, `reuse lint`, `check-docs`.
- Do not add `Co-Authored-By` lines to commits.
- Use `clap` `conflicts_with` for mutually exclusive flags — never rely on silent no-ops.

## Disabled regions

Users can protect blocks from formatting:

```cmake
# cmakefmt: off
set(SPECIAL   keep   this   exactly)
# cmakefmt: on
```

Also supported: `# cmake-format: off/on`, `# fmt: off/on`, `# ~~~`.

## Releasing

Three steps are **owned by `.github/workflows/prepare-release.yml`** and
must not be done by hand:

- bumping `Cargo.toml` `version` (and the `Cargo.lock` refresh that
  follows)
- stamping `CHANGELOG.md` — turning the working `## Unreleased` block into
  `## <version> — <date>`
- running `scripts/sync-changelog.py` so
  `docs/src/content/docs/changelog.md` mirrors `CHANGELOG.md`

The same workflow also copies `docs/public/schemas/latest/schema.json` to
`docs/public/schemas/v${VERSION}/schema.json`, creates the release
commit, tags, and pushes. `release.yml` then builds, verifies, and
publishes from the tag.

Before triggering the workflow:

- make sure the `## Unreleased` block in `CHANGELOG.md` is complete and
  attributed to the right semver-impact buckets
- run `cmakefmt config schema > docs/public/schemas/latest/schema.json`
  if any config-shape change landed since the previous release (CI's
  "Check JSON schema is up to date" gate enforces this anyway)
- add a v${VERSION} datapoint to
  `docs/src/components/VersionTrendChart.astro` with the wall-time +
  binary-size measurement and a one-line annotation

If a CI failure forces a re-run, prefer fixing the underlying issue and
re-triggering rather than hand-editing `Cargo.toml`/`CHANGELOG.md` to
match a partial release state.

## Dependencies (key crates)

- `pest`, `pest_derive` — PEG parser
- `pretty` — Wadler-Lindig pretty-printing
- `serde`, `serde_yaml_ng`, `toml` — config serialization
- `clap`, `clap_complete`, `clap_mangen` — CLI
- `thiserror` — error types
- `insta` — snapshot testing (dev)
- `walkdir`, `ignore` — file discovery
- `rayon` — parallel formatting
- `indicatif` — progress bars
- `regex` — pattern matching
- `tempfile` — temporary files (tests)
- `notify` — file watching (--watch)
- `lsp-server`, `lsp-types` — LSP support
