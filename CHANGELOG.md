# Changelog

This project follows a simple changelog discipline:

- keep user-visible changes in `Unreleased` until the next cut
- group entries by impact, not by file
- call out migration-impacting changes explicitly

## Unreleased

### Added

- `continuation_align` config knob — controls how continuation lines
  indent when a wrapped subkwarg group overflows `line_width`. Two
  modes: `under-first-value` (default — cmake-format-style hanging
  indent, aligned under the first value after the subkwarg) and
  `same-indent` (continuation at the subkwarg's own indent).
  Overridable per-command via `per_command_overrides` or per-spec
  via `layout.continuation_align`.

  **Note:** this is a behaviour change from v1.1.0. Before v1.2.0
  the formatter always used same-indent continuation (the knob did
  not exist). If an install/DIRECTORY section in your codebase has
  a subkwarg value list long enough to wrap (e.g.
  `PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE
  GROUP_READ`), its continuation lines will now align under the
  first value column instead of wrapping to the subkwarg indent.
  Set `continuation_align: same-indent` to restore the previous
  layout.
- `install(TARGETS ...)` artifact-kind subgroups are now modelled as
  nested keyword sections per CMake's documented signature. Each of
  `RUNTIME/LIBRARY/ARCHIVE/OBJECTS/FRAMEWORK/BUNDLE/PRIVATE_HEADER/
  PUBLIC_HEADER/RESOURCE/FILE_SET/CXX_MODULES_BMI` carries its own
  `DESTINATION/PERMISSIONS/CONFIGURATIONS/COMPONENT/NAMELINK_COMPONENT`
  subkwargs and `OPTIONAL/EXCLUDE_FROM_ALL/NAMELINK_ONLY/NAMELINK_SKIP`
  subflags. `OBJECTS` was previously missing; `FILE_SET` now correctly
  takes a positional set name.
- `install(PROGRAMS)` form (previously absent). `install(SCRIPT|CODE)`
  now accept `COMPONENT`, `ALL_COMPONENTS`, `EXCLUDE_FROM_ALL`.
  `install(RUNTIME_DEPENDENCY_SET)` fully modelled with per-artifact-
  kind subgroups plus the seven regex/file filter kwargs.
- `install(IMPORTED_RUNTIME_ARTIFACTS)` restructured with the same
  artifact-kind subgroup pattern as TARGETS.
- `install(FILES)` gains `TYPE`, `RENAME`, `EXCLUDE_FROM_ALL`.
- `install(DIRECTORY)` gains `TYPE`, `DIRECTORY_PERMISSIONS`,
  `CONFIGURATIONS`, `MESSAGE_NEVER`, `EXCLUDE_FROM_ALL`,
  `FILES_MATCHING`; `PATTERN`/`REGEX` subgroups now take their
  pattern as a positional and accept `EXCLUDE` plus a nested
  `PERMISSIONS` subkwarg.
- `RUNTIME_DEPENDENCIES` promoted from an unstructured value list to
  a proper kwarg group with `DIRECTORIES`, `PRE_INCLUDE_REGEXES`,
  `PRE_EXCLUDE_REGEXES`, `POST_INCLUDE_REGEXES`,
  `POST_EXCLUDE_REGEXES`, `POST_INCLUDE_FILES`,
  `POST_EXCLUDE_FILES` subkwargs.

### Changed

- Pair-aware vertical rendering for sections with nested subkwargs.
  When a wrapped section contains subkwargs, each `subkwarg + value`
  pair renders as a single logical line at the nested indent —
  matching the layout shown in `cmake --help-command install`.
  Non-grouped sections (`PUBLIC`/`PRIVATE`/`INTERFACE`, etc.) keep
  their existing flat packing.
- Trailing comments attached to a keyword header (`RUNTIME # runtime
  artifacts`) stay on the header line when they fit within
  `line_width`, and reflow through the standard comment formatter
  when they don't.

### Fixed

- Subkwarg argument values no longer collide with ancestor keyword
  names. `install(TARGETS foo LIBRARY COMPONENT Runtime)` previously
  reinterpreted `Runtime` as the `RUNTIME` artifact-kind subgroup;
  the splitter now force-attaches a kwarg's required nargs as
  values regardless of token classification. `OneOrMore` nargs
  (`CONFIGURATIONS Debug Runtime`) are also protected.
- Inline comments interleaved between a subkwarg and its value no
  longer count toward the subkwarg's nargs quota. The grouped
  writer advances by non-comment tokens so
  `COMPONENT # comment\n  Runtime` keeps `Runtime` as the
  `COMPONENT` value.
- `#` line comments appearing *before* a kwarg's required positional
  (e.g. `FILE_SET # set comment\n  HEADERS …`) no longer swallow the
  positional into the comment text. Positionals are always emitted
  before any comments on the header line.
- Long trailing comments on a keyword header now respect
  `line_width` — they break to a new line at the nested indent and
  reflow rather than producing an overlong line.
- `autosort` no longer scrambles kwarg sections whose spec declares
  nested subkwargs or flags. The gate applies equally to
  explicitly-`sortable: true` sections, so malformed specs can't
  bypass it. Pure flat-list kwargs (`PUBLIC`, user ITEMS-style
  lists) still sort.
- Removed an accidental top-level `PERMISSIONS` entry from
  `install(DIRECTORY)`. Per CMake docs, `PERMISSIONS` only appears
  as a subkwarg of a `PATTERN|REGEX` subgroup; the top-level
  permission kwargs are `FILE_PERMISSIONS` and
  `DIRECTORY_PERMISSIONS`.

### Removed

- Dormant `fuzz/` directory (three cargo-fuzz targets plus the
  frozen pest parser comparison harness). Never wired into CI; the
  parser rewrite they guarded is shipped. Can be recreated if a
  fuzzing push returns.

### Documentation

- Rustdoc / docs.rs surface polished: docs.rs now builds with
  `all-features` and the `docsrs` cfg so feature-gated items
  (`cli`-only re-exports, `lsp::run`, `wasm::format`) render with
  feature badges. Added module-level docs for `parser::ast`, a
  `# Features` table and expanded Organisation section at the crate
  root, explicit 1-based line/column guarantees on
  `ParseDiagnostic` and `FileParseError`, a "Loading from disk"
  section on `Config`, per-variant docs on `NArgs`,
  resolution-order notes on `CommandConfig`, and minimal doctest
  examples on `CommandRegistry::merge_toml_overrides` /
  `merge_yaml_overrides`.

### Internal

- Migrated the embedded built-in command spec from
  `src/spec/builtins.toml` to `src/spec/builtins.yaml`. The file
  shrank from 2433 to 2030 lines and the deeply-nested install
  form hierarchy is now legible at a glance. User config and spec
  override files still accept both TOML and YAML; only the
  embedded baseline format changed. Parse errors on the embedded
  spec now surface with YAML-native line/column metadata.

## 1.1.0 — 2026-04-22

### Added

- Formatting Cookbook page — common formatting goals with before/after
  examples and config links
- Glossary page — definitions of all terminology used in docs, config,
  and parse tree dumps
- "Why `cmakefmt`?" cards on the landing page
- Playground CTA after the Performance section
- Community sidebar section (badge, projects using `cmakefmt`)

### Changed

- Landing page restructured: explainer tagline, "Why" cards replace
  the old Features section, slimmed "Where to Go Next" with three
  sub-sections of two cards each
- Comparison page updated with parallelism, recursive discovery, config
  autocomplete, watch mode, and reordered table by user impact
- Mobile-friendly docs: responsive layouts, unified sidebar menu with
  TOC, chart view-transition support, "See all features" toggle on
  mobile
- Homepage animation: lazy panel initialization for production
  hydration, `magic-move` property compatibility fix
- "Getting Started" replaces "Installation" in the header nav

### Changed

- Real-world fixture corpus expanded with four large-file pins
  (`opencv_root`, `blender_root`, `llvm_libc_math`, `grpc_root`) so the
  per-file benchmark suite is fully reproducible from
  `tests/fixtures/real_world/manifest.toml`
- Refreshed headline performance numbers after the parser rewrite,
  lazy-diff work, and related micro-optimisations:
  - per-file geo-mean: **48× → 104×**
  - whole-repo geo-mean: **49× → 150×** parallel, **19× → 95×** serial
  - fastest whole-repo speedup: **485× → 2,853×** (opencv, 282 files, parallel)
  - 55K-line gRPC root `CMakeLists.txt`: **180 ms → 39 ms**
  - aggregate corpus wall time: **252 s → 0.65 s** (`--parallel`, 14 repos)

### Fixed

- Standalone comment immediately following an argument with a trailing
  comment no longer merges into that trailing comment on a second format
  pass. The vertical-arguments writer now refuses to inline-attach a
  comment when the previous line already ends in a trailing comment.
- All references to the non-functional `cmakefmt init` shorthand replaced
  with the canonical `cmakefmt config init` (landing page, getting-started,
  migration guide, CLI reference, and the binary's `--help` long text).
  The bare `cmakefmt init` form was already inert — the CLI parses `init`
  as a path argument — so docs that promised it were misleading.
- Diff computation skipped when result is unused — **14s → 0.6s** on a
  55K-line file. The Myers diff algorithm was running eagerly on every
  invocation even when `--diff` was not requested.
- `autosort` now activates on keyword sections containing inline
  comments — previously any inline comment silently disabled the
  heuristic
- Memoized sort keys to avoid repeated `to_ascii_lowercase()` per
  comparison
- Single-pass token classification (`classify_token`) eliminates
  redundant case conversion in the per-argument hot path
- Combined iterator passes in `try_format_inline` and
  `try_format_hanging` (two `.any()` calls merged into one)

## 1.0.0 — 2026-04-14

### Added

- `--list-unknown-commands` flag — parse files and report commands that
  don't match any built-in or user-defined spec, with file:line locations
- `--preview` flag — enable all experimental formatting options
- `--report-format edit` — editor-friendly JSON output with full-file
  replacements for each changed file
- `[experimental]` config section — placeholder for unstable formatting
  options gated behind `--preview` (no options yet)
- `enable_sort` config option — sort arguments in keyword sections marked
  `sortable` in the command spec (default: off)
- `autosort` config option — heuristically sort keyword sections where all
  arguments are simple unquoted tokens (default: off)
- `sortable` annotation for keyword specs — mark sections whose arguments
  may be sorted when `enable_sort` is enabled
- Stability contract published at `cmakefmt.dev/stability/`
- `wrap_after_first_arg` config option and layout hint — keep the first
  positional argument on the command line when wrapping. Enabled by default
  for `set()` so the variable name always stays on the `set(` line.
- Trailing inline comments now stay attached to their preceding argument
  in both packed and vertical layouts
- Trailing comment reflow — long trailing comments that exceed `line_width`
  are reflowed with continuation lines aligned to the `#`. The parser
  recognises column-aligned continuation comments and merges them back into
  the trailing comment on re-parse, ensuring idempotent round-trips.
- Remaining positional arguments are packed inline after the first argument
  when they fit within `line_width`, avoiding unnecessary vertical layout
- `cmakefmt dump ast` subcommand — print the raw parser AST as a
  colored Unicode box-drawing tree for debugging parser behavior
- `cmakefmt dump parse` subcommand — print a spec-resolved parse tree
  showing keyword/flag/positional grouping and flow-control nesting.
  Nested keyword specs are resolved recursively (e.g. `FORCE` shows as
  `FLAG` under `CACHE`).
- Demo GIF on the README and Getting Started page
- Weekly scheduled benchmark CI for consistent performance tracking
- Large-file LSP timeout test (2000 lines, < 1 second)

### Changed

- `reflow_comments` config option merged into `enable_markup` —
  `enable_markup: true` now controls both markup handling and comment
  reflow. The standalone `reflow_comments` key is no longer accepted in
  native config files (legacy conversion still maps it).
- `set()` CACHE spec updated: `STRING`/`FILEPATH`/etc. type argument uses
  `nargs = 2` and `FORCE` is a flag at the `CACHE` level rather than a
  nested positional argument
- Semantic verifier strips all comments from comparison — comments have no
  CMake semantic meaning and were causing false-positive verification
  failures when comment reflow changed their structure
- Default config example uses `my_add_test` with VERBOSE flag, matching
  the playground source
- Playground loads default config from WASM `default_config_yaml()` at
  runtime instead of a hardcoded string
- README banner uses absolute URL for PyPI/crates.io rendering

### Fixed

- `autosort` now activates on keyword sections that contain inline
  comments — previously, the presence of any inline comment caused the
  heuristic to skip the section entirely

## 0.10.0 — 2026-04-12

### Added

- `--explain` flag — show per-command formatting decisions (layout choice,
  config values, thresholds) for a single file
- `--watch` flag — watch directories for changes and reformat in-place
  automatically; press Ctrl+C to stop
- `.editorconfig` fallback — when no `.cmakefmt.yaml` is found, `indent_style`
  and `indent_size` from `.editorconfig` are used as defaults. Disable with
  `--no-editorconfig`
- `--debug` now logs discovery context: active ignore sources, `--path-regex`
  filter, `--staged`/`--changed` mode, and files filtered by `--path-regex`

### Changed

- `--progress-bar` no longer requires `--in-place` — works with `--check`,
  `--summary`, `--quiet`, and non-human report formats. Automatically
  suppressed when stdout streams to the terminal, with a warning explaining why
- Stable Rust library API for embedding: parser internals are no longer part
  of the public surface, parse/config/spec failures now use crate-owned error
  types, and `CommandConfig` no longer exposes internal representation fields
- PyPI package now includes README as the long description on the project page

## 0.9.0 — 2026-04-12

### Migration

- `pip install cmakefmt` is now CLI-only. It installs the `cmakefmt`
  executable into your environment rather than a Python module.
- The `v0.8.0` Python binding API is gone in `v0.9.0`, so `import cmakefmt`
  no longer works on the new release line.

### Added

- `conda install -c conda-forge cmakefmt` — now available on conda-forge
- Animated before/after formatting demo on the landing page
- sdist smoke test in CI — verifies source distribution installs correctly
- Native aarch64 Linux wheel builds via `ubuntu-24.04-arm` (no more
  cross-compilation)

### Changed

- **Breaking:** `pip install cmakefmt` now installs the CLI binary instead of
  a Python library. `import cmakefmt` no longer works — use the binary directly.
- `--fast` renamed to `--no-verify` (`--fast` remains as a deprecated alias)
- `--colour` renamed to `--color` (`--colour` remains as an alias)

### Fixed

- `--quiet` / `-q` now suppresses formatted output in stdout mode (previously
  only suppressed "would be reformatted" lines in `--check` mode)
- `pyproject.toml` now included in sdist (fixes source builds on platforms
  without pre-built wheels)
- PyPI wheel smoke test now runs on macOS aarch64 (previously skipped)

## 0.8.0 — 2026-04-11

### Added

- Python bindings via PyO3 — `pip install cmakefmt` exposes
  `format_source()`, `is_formatted()`, and `default_config()` as native
  Python functions with the same config schema as `.cmakefmt.yaml`
- Python `config` parameter accepts both YAML strings and Python dicts
- `Config::from_yaml_str()` public API for parsing config from YAML
  strings through the validated `FileConfig` schema

### Changed

- **Breaking:** `command_case` and `keyword_case` moved from `style:` to
  `format:` section in config files. The `style:` section is removed.

### Fixed

- Broken pipe error when piping output to `head` or `less`
- WASM binding now validates config through `FileConfig` schema instead
  of silently accepting invalid fields via the flat `Config` struct

## 0.7.0 — 2026-04-11

### Added

- `--summary` flag — shows per-file status lines with change details, line
  counts, and formatting time; suppresses formatted output in stdout mode
- `--sorted` flag — sorts discovered files by path before processing for
  deterministic alphabetical output order
- Short flags: `-s` (summary), `-q` (quiet), `-d` (diff), `-p` (progress-bar)
- Streaming output — per-file results now appear as each file completes
  instead of batching until the end, including in parallel mode
- Deterministic output order — parallel results are buffered and flushed
  in input order, so output is stable across runs
- Unclosed parenthesis diagnostics — parse errors now point to the
  unmatched `(` instead of the end of the file
- CodSpeed CI benchmark tracking for automated performance regression
  detection on every PR

### Changed

- Parallel formatting is now the default — uses available CPUs minus one
  (minimum 1). Use `--parallel 1` to force serial processing.

## 0.6.0 — 2026-04-10

### Removed

- `--lsp` flag — use `cmakefmt lsp` instead
- `--generate-completion` flag — use `cmakefmt completions <SHELL>` instead
- `--dump-config` flag — use `cmakefmt config dump` instead
- `--dump-schema` flag — use `cmakefmt config schema` instead
- `--check-config` flag — use `cmakefmt config check` instead
- `--show-config` flag — use `cmakefmt config show` instead
- `--show-config-path` / `--find-config-path` flag — use `cmakefmt config path`
  instead
- `--explain-config` flag — use `cmakefmt config explain` instead
- `--convert-legacy-config` / `--convert-legacy-config-format` flags — use
  `cmakefmt config convert` instead
- `cmakefmt init` (top-level subcommand) — use `cmakefmt config init` instead

### Added

- `cmakefmt config show`, `cmakefmt config path`, and `cmakefmt config explain`
  now accept an optional file path argument directly (e.g.
  `cmakefmt config show src/CMakeLists.txt`)
- File-not-found validation for `config show`, `config path`, and
  `config explain` — clear error message when the target file does not exist

### Changed

- `cmakefmt config dump` and `cmakefmt config show` format is now specified
  via `--format` flag (e.g. `cmakefmt config dump --format toml`) instead of
  a positional argument
- 78 `conflicts_with` annotations removed from the CLI definition — subcommand
  structure now handles mutual exclusivity

## 0.5.0 — 2026-04-10

### Added

- `cmakefmt config` subcommand group — `dump`, `schema`, `check`, `show`,
  `path`, `explain`, `convert`, and `init` sub-subcommands for config
  inspection and conversion
- `cmakefmt lsp` subcommand (replaces `--lsp` flag, which is now deprecated)
- `cmakefmt completions <SHELL>` subcommand (replaces `--generate-completion`
  flag, which is now deprecated)
- `cmakefmt install-hook` subcommand — one-command git pre-commit hook setup
- LSP: `workspace/didChangeConfiguration` support — live config reload when
  `.cmakefmt.yaml` changes without restarting the server
- LSP: `textDocument/codeAction` — "Disable cmakefmt for selection" action
  that inserts `# cmakefmt: off/on` barriers
- LSP: 10-second timeout on formatting requests to prevent pathological inputs
  from freezing the editor
- Colored CLI help output (green headers, cyan flags)
- Cargo-fuzz targets for parser and formatter (`fuzz/`)
- `cmakefmt-fix` pre-commit hook for auto-formatting (in addition to the
  existing check-only `cmakefmt` hook)
- Docker image published to GHCR (`ghcr.io/cmakefmt/cmakefmt`) on every
  release
- Cross-platform output consistency CI workflow
- Real-world regression suite CI workflow (CMake, LLVM, OpenCV)
- SBOM generation (`cargo-cyclonedx`) in the release workflow
- Docker build CI workflow
- WASM API documentation page
- Docs site redesign: Plus Jakarta Sans headings, gradient hero with animated
  dot grid, animated link underlines, card lift-on-hover, sidebar active
  indicator, page load fade-in
- Benchmarks for config pattern validation, legacy conversion, and atomic
  writes

### Changed

- Unrecognized config keys now produce an error instead of being silently
  ignored (`deny_unknown_fields` on all config sections)
- `require_valid_layout` error now suggests the specific `line_width` value
  needed to accommodate the offending line
- Config regex patterns (`literal_comment_pattern`, `explicit_trailing_pattern`,
  etc.) are now compiled once per formatting run instead of once per comment
  or command, improving performance on comment-heavy files

### Deprecated

- `--lsp` flag — use `cmakefmt lsp` instead
- `--generate-completion` flag — use `cmakefmt completions <SHELL>` instead
- `--dump-config` flag — use `cmakefmt config dump` instead
- `--dump-schema` flag — use `cmakefmt config schema` instead
- `--check-config` flag — use `cmakefmt config check` instead
- `--show-config` flag — use `cmakefmt config show` instead
- `--show-config-path` flag — use `cmakefmt config path` instead
- `--explain-config` flag — use `cmakefmt config explain` instead
- `--convert-legacy-config` flag — use `cmakefmt config convert` instead
- `cmakefmt init` (top-level) — use `cmakefmt config init` instead

## 0.4.0 — 2026-04-09

### Added

- `cmakefmt init` subcommand — generates a starter `.cmakefmt.yaml` in the
  current directory
- `--check-config` flag — validates a config file and exits without formatting
- `--stat` flag — prints a git-style summary (`3 files changed, 12 lines
  reformatted`)
- Elapsed time shown in the formatting summary (e.g. `in 0.42s`)
- Fix hint printed when `--check` fails (`hint: run cmakefmt --in-place .`)
- User-friendly panic handler with structured bug report template
- `.pre-commit-hooks.yaml` for pre-commit integration
- `Dockerfile` for CI usage
- GitHub issue templates for bug reports and feature requests
- Architecture guide for contributors
- FAQ docs page
- LSP-mode editor configs for Neovim, Helix, and Zed
- Azure Pipelines and Bitbucket Pipelines examples in CI docs
- Migration guide expanded with key differences and unsupported options tables

### Fixed

- `--diff` now works with `--check` and non-human `--report-format` modes;
  previously both suppressed the unified diff output

### Security

- Config regex patterns (`literal_comment_pattern`, `explicit_trailing_pattern`,
  `fence_pattern`, `ruler_pattern`) are now validated at config load time;
  previously invalid or pathological regexes were silently accepted and could
  cause CPU exhaustion (ReDoS)
- `--in-place` writes are now atomic (write to temp file, then rename);
  previously a TOCTOU race could cause unintended overwrites if the target
  file was replaced with a symlink between read and write

## 0.3.0 — 2026-04-08

### Added

- `--dump-schema` flag — prints the JSON Schema for the `.cmakefmt.yaml` /
  `.cmakefmt.toml` config file to stdout and exits; schema is also published
  at `cmakefmt.dev/schemas/latest/schema.json` for zero-config YAML
  autocomplete in editors with `redhat.vscode-yaml` or similar plugins
- `--lsp` flag — starts a stdio JSON-RPC Language Server Protocol server
  supporting `textDocument/formatting` and `textDocument/rangeFormatting`;
  enables format-on-save in any editor with LSP client support (Neovim,
  Helix, Zed, Emacs, …) without a dedicated extension
- Guide pages on [cmakefmt.dev](https://cmakefmt.dev): editor integration,
  CI integration, tool comparison, badge, and "Projects using cmakefmt"

## 0.2.0 — 2026-04-07

### Added

- interactive browser playground at [cmakefmt.dev/playground](https://cmakefmt.dev/playground/) —
  format CMake code, edit config, and define custom command specs entirely in
  the browser via WebAssembly
- `format.disable` config option — global kill-switch that returns the source
  file unchanged; useful for temporarily opting out of formatting without
  removing the config file
- `format.line_ending` config option — controls output line endings: `unix`
  (LF, default), `windows` (CRLF), or `auto` (detects predominant ending in
  the input and preserves it)
- `format.always_wrap` config option — list of command names that are always
  rendered in vertical (wrapped) layout, never inline or hanging; the
  `always_wrap` flag in per-command specs (`commands:`) is now also honoured
- `format.require_valid_layout` config option — when `true`, the formatter
  returns an error if any output line exceeds `line_width`; useful for strict
  CI enforcement
- `format.fractional_tab_policy` config option — controls sub-tab-stop
  indentation remainders when `use_tabs` is `true`: `use-space` (default)
  keeps them as spaces, `round-up` promotes them to a full tab
- `format.max_rows_cmdline` config option — maximum number of rows a
  positional argument group may occupy before the hanging-wrap layout is
  rejected and vertical layout is used instead (default: `2`)
- `markup.explicit_trailing_pattern` config option — regex pattern (default
  `#<`) that marks an inline comment as trailing its preceding argument,
  keeping it on the same line rather than wrapping to a new line

## 0.1.1 — 2026-04-06

### Added

- Homebrew installation support (`brew install cmakefmt/cmakefmt/cmakefmt`)
- shell completion installation instructions
- site metadata and crate status badge on [docs.rs](https://docs.rs)

### Changed

- improved [docs.rs](https://docs.rs) readability and tightened public API surface
- documentation clarity and wording improvements

## 0.1.0 — 2026-04-05

### Added

- full CLI workflow: `--check`, `--diff`, `--in-place`, `--staged`,
  `--changed`, `--files-from`, `--parallel`, `--dump-config`, `--list-input`,
  `--list-changed`, `--explain-config`, `--quiet`, `--keep-going`
- recursive file discovery with `.cmakefmtignore` and `--exclude-regex` support
- YAML and TOML config file support with automatic discovery
- comment preservation and fence/barrier support (`# cmakefmt: off/on`)
- pragma-gated rollout mode
- formatter result caching
- colored diff output and in-place progress bar
- CI-oriented report formats (JSON, JUnit, SARIF, GitHub Actions, GitLab CI)
- legacy `cmake-format` config conversion (`--convert-config`)
- built-in and module-command spec coverage audited against CMake 4.3.1
- custom command specifications via config
- real-world regression corpus covering LLVM, Qt, protobuf, and more
- performance benchmarks: ~20× geometric-mean speedup over `cmake-format`
- parallel formatting with `--parallel`
- comprehensive docs site at [cmakefmt.dev](https://cmakefmt.dev)
- shell completion generation (`--completions`)
- dual MIT/Apache-2.0 licensing with full REUSE compliance
- Windows, macOS, and Linux support

### Compatibility Notes

- `cmakefmt` aims to be easy to migrate to from `cmake-format`, but output is
  not intended to be byte-for-byte identical
- config option names differ from `cmake-format` in places; use
  `--convert-config` to migrate

## Release Process

For each release:

1. move relevant `Unreleased` entries into a versioned heading
2. summarize major user-visible changes
3. note any compatibility or migration impact
4. link the release tag or GitHub release when published
