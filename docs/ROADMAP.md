# Roadmap

Each phase has clear acceptance criteria. Do not move to the next phase
until all criteria for the current phase are met.

---

## Phase 1 — Scaffold + Grammar

**Goal**: A Rust project that can parse any valid CMake file without crashing.

### Tasks

- [x] `cargo new cmakefmt --lib` + binary target in `Cargo.toml`
- [x] Add all dependencies (pest, pretty, clap, serde, toml, thiserror, insta, criterion)
- [x] Write `src/parser/cmake.pest` — full CMake grammar covering:
  - File, command invocation
  - Bracket arguments `[=[...]=]`
  - Quoted arguments with escape sequences
  - Unquoted arguments
  - Line comments `# ...`
  - Bracket comments `#[=[ ... ]=]`
  - Variable references `${...}`, `$ENV{...}`, `$CACHE{...}`
  - Generator expressions `$<...>` (treat as opaque unquoted content)
  - Continuation lines (backslash at end of line inside arguments)
- [x] Write `src/parser/ast.rs` — CST → AST conversion
- [x] Unit tests: parse every construct, assert no panics, check node types

### Acceptance criteria

- [x] All fixture files in `tests/fixtures/` parse without error
- [x] The following real-world files parse correctly:
  - CMake's own `CMakeLists.txt`
  - LLVM's top-level `CMakeLists.txt`
  - A Qt module `CMakeLists.txt`
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes

---

## Phase 2 — Command Spec Registry

**Goal**: The formatter knows the argument structure of every CMake built-in command.

### Tasks

- [x] Define `src/spec/mod.rs` — `NArgs`, `PosSpec`, `KwargSpec`, `CommandForm`, `CommandSpec`
- [x] Create `src/spec/builtins.toml` — specs for all ~150 CMake built-in commands,
      covering at minimum:
  - Core: `cmake_minimum_required`, `project`, `set`, `unset`, `message`
  - Targets: `add_executable`, `add_library`, `add_custom_target`, `add_custom_command`
  - Properties: `set_target_properties`, `get_target_property`, `target_compile_options`,
    `target_compile_definitions`, `target_include_directories`, `target_link_libraries`,
    `target_link_options`, `target_sources`
  - Find: `find_package`, `find_library`, `find_path`, `find_file`, `find_program`
  - File system: `file`, `configure_file`, `include`, `add_subdirectory`
  - Install: `install`, `export`
  - Control flow: `if`/`elseif`/`else`/`endif`, `foreach`/`endforeach`,
    `while`/`endwhile`, `function`/`endfunction`, `macro`/`endmacro`, `return`
  - Misc: `list`, `string`, `math`, `option`, `include_guard`, `cmake_language`,
    `block`/`endblock`, `execute_process`, `external_project_add`
- [x] Implement `src/spec/registry.rs` — loads builtins + merges user overrides
- [x] Unit tests: registry returns correct spec for a variety of commands;
      returns fallback spec for unknown commands

### Acceptance criteria

- [x] `registry.get("target_link_libraries")` returns a spec with PUBLIC/PRIVATE/INTERFACE kwargs
- [x] `registry.get("install")` returns a `Discriminated` spec
- [x] `registry.get("my_unknown_command")` returns the default fallback spec (no crash)
- [x] All built-in specs deserialise from TOML without error
- [x] User config entries merge correctly with built-ins

---

## Phase 3 — Basic Formatter (no comments, no config)

**Goal**: Produce correctly formatted output for comment-free CMake files.
Uses the command spec registry from Phase 2 to drive keyword-aware grouping.

### Tasks

- [x] Implement `src/formatter/node.rs` — AST → Doc IR
- [x] Implement argument list layout (inline vs multi-line, see ARCHITECTURE.md)
- [x] Implement blank line preservation between statements
- [x] Wire up `src/lib.rs`: `format_source(src: &str, config: &Config) -> Result<String>`
- [x] Snapshot tests for basic formatting cases
- [x] Idempotency tests (`tests/idempotency.rs`)

### Acceptance criteria

- [x] All snapshot tests pass
- [x] All fixtures pass idempotency: `format(format(x)) == format(x)`
- [x] Line length limit is respected for all fixture files
- [x] Formatter does not alter the semantic meaning of any file
  (parse tree of formatted output equals parse tree of input, modulo whitespace)

---

## Phase 4 — Comment Preservation

**Goal**: Comments are preserved in correct positions.

### Tasks

- [x] Ensure pest grammar captures comments as positioned tokens
- [x] Implement `src/formatter/comment.rs` — comment → Doc IR conversion
- [x] Handle all comment positions:
  - Comment on its own line (before a command)
  - Trailing comment on same line as a command or argument
  - Comment inside an argument list (between arguments)
  - Comment after closing paren
  - Bracket comments spanning multiple lines
- [x] Add comment fixtures to `tests/fixtures/comments/`
- [x] Snapshot tests for all comment positions

### Acceptance criteria

- [x] No comment is ever lost or moved to a semantically different position
- [x] All comment snapshot tests pass
- [x] Idempotency holds for all comment fixtures

---

## Phase 5 — Configuration

**Goal**: Formatter behaviour is fully configurable via user config files.

### Tasks

- [x] Define `Config` struct with all options (see ARCHITECTURE.md)
- [x] Implement config file loading + directory-walk resolution
- [x] Implement CLI flag overrides
- [x] `command_case` option (lower/upper/unchanged)
- [x] `keyword_case` option (lower/upper/unchanged)
- [x] `line_length` option
- [x] `tab_size` option
- [x] `use_tabs` option
- [x] `max_empty_lines` option
- [x] `dangle_parens` + `dangle_align` options
- [x] `space_before_control_paren` option
- [x] `space_before_definition_paren` option
- [x] `max_hanging_wrap_lines`, `max_hanging_wrap_positional_args`, `max_hanging_wrap_groups` options
- [x] `min_prefix_length`, `max_prefix_length` options
- [x] `[markup]` section: `enable_markup`, `first_comment_is_literal`,
      `literal_comment_pattern`, `bullet_char`, `enum_char`,
      `fence_pattern`, `ruler_pattern`, `hashruler_min_length`,
      `canonicalize_hashrulers`
- [x] `[per_command_overrides.<name>]` overrides — let users tune any option per-command
      (e.g. always uppercase `SET`, wider lines for `message`)
- [x] Tests for config resolution precedence

### Acceptance criteria

- [x] All config options are exercised by at least one test
- [x] Config file not found → built-in defaults used (no error)
- [x] Invalid TOML → clear error message with file path and line number

---

## Phase 6 — CLI

**Goal**: A fully usable command-line tool.

### Tasks

- [x] Implement `src/main.rs` with `clap` derive API
- [x] Commands/flags:
  - `cmakefmt [FILE]...` — format and print to stdout
  - `cmakefmt` — recursively find CMake files under the current working directory
  - `cmakefmt -i [FILE]...` — format in-place
  - `cmakefmt --check [FILE]...` — exit 1 if any file would change
  - `cmakefmt --list-files [FILE|DIR]...` — list files that would be reformatted
  - `cmakefmt --path-regex <REGEX>` — filter recursively discovered CMake files
  - `cmakefmt -` — read from stdin, write to stdout
  - `cmakefmt --dump-config` — print a default config template
  - `cmakefmt --convert-legacy-config <PATH>` — convert old `cmake-format` config files to `.cmakefmt.toml`
  - `cmakefmt --config-file <PATH>` — one or more explicit config files
  - `cmakefmt --line-width <N>` — override config
  - `cmakefmt --version` — print version
- [x] Correct exit codes (0 = ok, 1 = check failed, 2 = error)
- [x] Helpful error messages (file path + line:col for parse errors)
- [x] Integration tests for CLI behaviour

### Acceptance criteria

- [x] `cmakefmt --check` in CI workflow on a correctly formatted file returns 0
- [x] `cmakefmt --check` on an unformatted file returns 1 (not 2)
- [x] `-i` modifies file and leaves it idempotent
- [x] Formatting 100 files in one invocation works correctly

---

## Phase 7 — Real-World Validation

**Goal**: The formatter produces reasonable output on real CMake projects.

### Tasks

- [x] Collect 10+ real-world `CMakeLists.txt` files from popular projects
  (CMake, LLVM, Qt, OpenCV, Boost, abseil, googletest, etc.)
- [x] Add them to `tests/fixtures/real_world/`
- [x] Run formatter on all of them; manually review output
- [x] Fix any formatting regressions found
- [x] Snapshot all real-world outputs

### Acceptance criteria

- [x] All real-world files pass idempotency
- [x] Formatted output is reviewed and judged reasonable by a human
- [x] No panics or errors on any real-world input

---

## Phase 8 — Bottlenecks

**Goal**: Understand and remove the main performance bottlenecks.

### Tasks

- [x] Add `criterion` benchmarks for parsing and formatting
- [x] Extend the benchmark suite to cover each major `cmakefmt` capability
  (parser, formatter, CLI file discovery, config loading, check mode, in-place mode)
- [x] Build a dedicated benchmark corpus with representative inputs:
  - small synthetic files for parser/layout microbenchmarks
  - medium real-world files from `tests/fixtures/real_world/`
  - at least one large stress file (~1000+ lines)
  - command-heavy cases (`install`, `file`, `string`, `list`, target commands)
  - comment-heavy files and barrier/fence-heavy files
- [x] Benchmark each major internal path separately and keep the benchmark names stable:
  - parser only
  - parser + AST construction
  - formatter only from parsed AST
  - end-to-end `format_source`
  - config discovery/loading
  - file discovery/filtering
  - check-mode path
  - in-place write path
  - debug-mode overhead
  - opt-in parallel execution overhead and speedup
- [x] Add benchmark baselines and compare regressions over time:
  - save a local baseline before major formatter changes
  - compare bench runs against the saved baseline
  - document the expected benchmark commands in `README.md`
- [x] Investigate and quantify the hottest bottlenecks:
  - parser time
  - AST construction time
  - command-spec lookup overhead
  - layout/packing heuristics in `src/formatter/node.rs`
  - comment/barrier handling overhead
  - allocation hotspots and repeated string building
- [x] Profile with `cargo flamegraph` and/or equivalent profilers when benchmarks are unexpectedly slow
  - capture parser-only flamegraphs
  - capture formatter-only flamegraphs
  - capture end-to-end flamegraphs on a large real-world file
  - summarize the top hotspots and the changes made to address them
- [x] Optimize the hottest paths aggressively but safely:
  - reduce avoidable allocations/cloning
  - reduce repeated case conversions / string normalization where measurable
  - reuse parsed/loaded data where appropriate
  - simplify expensive formatting heuristics that do not materially improve output
  - verify that optimizations do not regress semantics, idempotency, or diagnostics
- [x] Measure scaling behavior across realistic workloads:
  - single-file latency on small, medium, and large files
  - total throughput across many files
  - startup overhead for short invocations
  - opt-in parallel speedup versus single-threaded execution
  - memory footprint under serial and parallel modes
- [x] Hit the performance target: format a 1000-line `CMakeLists.txt` in < 10ms
- [x] Document the final benchmark methodology and results in the repo
  - benchmark environment details
  - hardware / OS notes
  - corpus description
  - commands used to reproduce results
- [x] Run the head-to-head `cmakefmt` vs `cmake-format` benchmark and publish results in `README.md`
  - Install `cmake-format` (`pip install cmakelang`) in the benchmark environment
  - Run both tools against every file in `tests/fixtures/real_world/`
  - Measure wall-clock time (hyperfine or criterion shell benchmark)
  - Report speedup factor for each file and overall geometric mean
  - Include results table in `README.md`
- [x] Add `cmakefmt --debug` mode to print config resolution, file discovery matches,
      parser locations, and formatter layout decisions for diagnostics
- [x] Add `cmakefmt --parallel [<JOBS>]` to parallelise file formatting when explicitly requested
      while keeping the default execution mode single-threaded
- [x] Respect `cmake-format: off` / `cmake-format: on` barriers and `# ~~~` fence regions,
      and support `cmakefmt: off` / `cmakefmt: on` as native aliases
- [x] Add pre-commit hook config example (`.pre-commit-config.yaml`)
- [x] Add GitHub Actions CI (test + clippy + fmt on Linux/macOS)

### Acceptance criteria

- [x] Benchmark target met (< 10ms per 1000-line file)
- [x] `cmakefmt` is measurably faster than `cmake-format` on every real-world fixture
- [x] Benchmark methodology and reproduction commands are documented
- [x] At least one profiling pass was performed on the hottest workloads and acted on
- [x] Parallel mode speedup and memory impact are measured and recorded
- [x] CI passes on Linux and macOS

---

## Phase 9 — Completeness

**Goal**: Fill out the supported command surface so the formatter behaves well across the full CMake ecosystem we target.

### Tasks

- [x] Audit `src/spec/builtins.toml` to a full built-in and supported module-command surface,
      including the utility/deprecated modules we intend to recognize and format well

---

## Phase 10 — User Friendliness

**Goal**: Make the project easy to adopt, understand, and contribute to.

### Tasks

- [x] Define the published documentation structure and navigation for GitHub Pages
  - landing page / project overview
  - installation page
  - CLI usage reference
  - configuration reference
  - formatter behavior guide
  - migration / compatibility guide
  - library API guide
  - architecture / implementation overview
  - changelog / release notes
- [x] Expand the user-facing docs into a full guide set
  - installation from Cargo and supported package managers
  - quick-start workflow for local formatting, `--check`, and `-i`
  - file discovery, `--path-regex`, `--list-files`, and `--parallel`
  - barriers and fence regions
  - debug mode and troubleshooting
  - benchmark / performance guide
- [x] Write a complete CLI reference
  - every flag with behavior, defaults, and exit-code semantics
  - examples for single-file, recursive, stdin, in-place, and CI usage
  - examples for config overrides, `--dump-config`, and `--convert-legacy-config`
  - note which flags are operational-only versus config-backed
- [x] Write a complete configuration reference
  - document every config section and option
  - show the built-in defaults
  - explain config resolution precedence
  - explain per-command overrides
  - explain which markup features are opt-in and what they do today
- [x] Write a formatter behavior guide
  - blank-line preservation
  - comment preservation and comment reflow behavior
  - control-flow indentation rules
  - horizontal wrap versus vertical layout heuristics
  - barriers / disabled regions / fence passthrough
  - known formatting differences from `cmake-format`
- [x] Write a migration and compatibility guide for `cmake-format` users
  - command-line invocation mapping
  - config-file compatibility and differences
  - unsupported or intentionally different options
  - output-style differences that users should expect
  - rollout advice for CI / pre-commit migration
- [x] Write a library/API guide for embedders
  - public crate entry points
  - config construction and override flow
  - error model
  - examples for `format_source` and related helpers
  - expectations around stability before and after `1.0`
  - thorough rustdoc coverage for the public crate surface
  - keep `cargo doc --no-deps` green
- [x] Keep an explicit changelog and release-note process
  - choose the changelog format
  - document categories for user-visible changes
  - link releases to changelog entries
  - ensure breaking changes and migration notes are called out clearly
- [x] Add directory-level contributor readmes explaining the repo structure
  - `src/`
    - parser, formatter, config, spec, and CLI responsibilities
  - `tests/`
    - purpose of each Rust test module
  - `tests/fixtures/`
    - fixture categories and how to add/update them
  - `tests/snapshots/`
    - what snapshot files represent and when they should change
  - `benches/`
    - benchmark intent and how to extend the suite
  - `docs/`
    - where long-form docs live and how to keep them aligned
- [x] Add doc maintenance rules so docs stay in sync with code
  - update `CONTRIBUTING.md` with doc update requirements where needed
  - identify which changes must also update `README.md`, docs pages, and changelog
  - add lightweight validation for docs where practical (link checks or doc build)
- [x] Publish the docs site through GitHub Pages
  - choose the site generator / layout
  - add the build/deploy workflow
  - ensure local preview instructions exist
  - ensure versioned or release-tagged docs policy is clear
- [ ] Do a full documentation overhaul before alpha
  - rewrite `docs/ARCHITECTURE.md` so it clearly explains how parsing, command-spec resolution,
    layout selection, comment preservation, barriers, and CLI execution fit together
  - expand the published architecture page to explain the pipeline in user-facing terms,
    not just as a short layer list
  - massively expand the API guide with realistic examples for:
    - `format_source`
    - custom `Config` construction
    - loading config from disk
    - using `CommandRegistry` overrides
    - collecting debug output
    - embedding `cmakefmt` in editors, scripts, or CI tools
    - handling errors cleanly
  - expand the behavior guide with concrete before/after examples for:
    - line wrapping
    - keyword grouping
    - comment preservation
    - comment reflow
    - blank-line preservation
    - disabled regions / fences
    - control-flow indentation
    - per-command overrides
    - known non-goals / intentional differences from `cmake-format`
  - expand `site/src/cli.md` so the "Common Examples" section shows example output
    for flags whose effect is easy to visualize
  - thoroughly rewrite `site/src/config.md`
    - provide a table of contents for all config options
    - give each option a dedicated subsection
    - explain what it does, when to use it, its default, and a small example where useful
    - clearly separate global formatting options, style options, markup options,
      per-command overrides, and custom command specs
  - rewrite `site/src/install.md` into a fuller install and adoption guide
    - local build vs `cargo install`
    - first-run checks
    - config bootstrap
    - upgrade/uninstall notes
    - pre-commit / CI usage
    - local docs preview
  - rewrite the docs landing page to sell the project more clearly
    - clear positioning statement
    - "why use cmakefmt?" summary
    - quick-start section that stays easy to find
    - benchmark / performance highlights that are honest and reproducible
    - migration callout for `cmake-format` users without disparaging the original tool
  - add stronger cross-linking and navigation between architecture, API, behavior,
    CLI, config, install, migration, and performance pages
  - add more troubleshooting / FAQ-style guidance where users are likely to get stuck
    (config discovery, ignored files, parsing failures, editor usage, migration surprises)
  - make benchmark claims in the docs traceable to `docs/PERFORMANCE.md`
    and keep them phrased as measured results rather than hype
  - ensure every major docs page answers:
    - what is this for?
    - what should I do first?
    - what are the common pitfalls?
    - where do I go next?

### Acceptance criteria

- [x] A new user can install `cmakefmt`, format a project, and understand the major CLI/config features from the docs alone
- [x] A `cmake-format` user can follow a migration guide and understand compatibility gaps before switching
- [x] Every public CLI flag and config option is documented in exactly one primary reference page
- [x] Contributor readmes exist for the main repo areas and explain where new code, fixtures, snapshots, and docs belong
- [x] The GitHub Pages docs site builds cleanly and is linked from `README.md`
- [x] User-visible changes have a documented changelog/release-note path
- [x] Public library APIs and exported data structures are documented well enough to generate useful API docs with `cargo doc --no-deps`
- [ ] The landing page makes a compelling, evidence-backed case for trying `cmakefmt`
- [ ] The architecture, API, behavior, CLI, config, and install pages all contain enough concrete examples
      that a user can solve common tasks without reading the source
- [ ] Config docs are navigable enough that a user can jump directly to any single option and understand its effect quickly

---

## Phase 11 — Debugging Clarity

**Goal**: Make failures and surprising behavior easy to understand, reproduce, and fix.

### Tasks

- [x] Redesign parser error rendering so the primary message is human-oriented
      and the raw parser expectation set is secondary detail
  - short summary first, e.g. "failed to parse quoted argument"
  - raw pest expectations only after the summary / hint
  - avoid dumping low-signal parser internals before the likely cause
- [x] Include richer source context in all parse/format/config diagnostics
  - absolute file path
  - 1-based line and column
  - 2-3 surrounding source lines when available
  - a caret or highlight at the failing span
  - a compact "while parsing/formatting ..." context line
- [x] Add targeted parser failure heuristics for common CMake mistakes
  - malformed quoted arguments / escaped quotes
  - bracket argument and bracket comment delimiter mismatches
  - unbalanced parens in command invocations and control-flow conditions
  - top-level template/configure-file placeholder issues in `.cmake.in`
  - likely earlier-line desync causing the reported failure to appear late
- [x] Make barrier/fence-related failures and surprises explicit
  - report when syntax is being passed through unchanged because formatting is disabled
  - explain when invalid syntax is tolerated only because it sits inside an off/fence region
  - include barrier state in diagnostics when it materially affects behavior
- [x] Improve formatter/layout diagnostics for "why did this wrap?" cases
  - report the chosen layout family (inline / hanging-wrap / vertical / preserved)
  - report the key thresholds that triggered the decision
    - `line_width`
    - `max_hanging_wrap_lines`
    - `max_hanging_wrap_positional_args`
    - `max_hanging_wrap_groups`
  - report the command-spec form selected from the registry
  - report whether source grouping preservation affected the result
- [x] Expand `--debug` into a structured troubleshooting mode useful on real repositories
  - file discovery summary
  - chosen config files and precedence order
  - effective per-command config for the current command
  - selected command-spec form and recognized keywords/flags
  - barrier/fence transitions
  - per-command layout decision summaries
  - final "why this file changed" summary where practical
- [x] Make config-file diagnostics more actionable
  - unknown key suggestions for close matches
  - point to the exact config file that introduced the bad value
  - explain precedence when multiple config files are in play
  - call out unsupported legacy keys and recommend `--convert-legacy-config` when relevant
- [x] Make CLI validation failures point to likely fixes
  - conflicting flag explanations
  - invalid regex diagnostics with the offending pattern
  - invalid `--colour` / enum values shown with allowed values
  - invalid `--progress-bar` usage explains the `--in-place` requirement
- [x] Add reproducibility and triage affordances
  - a compact repro recipe in error output when a file path is known
  - document a standard bug-report checklist in the user docs
  - document how to capture `--debug` output for issue reports
  - ensure failures are copy-paste friendly from CI logs
- [x] Add regression coverage specifically for diagnostics quality
  - parser failure snapshots
  - config failure snapshots
  - CLI validation failure snapshots
  - custom command-spec / registry failure cases
  - debug-mode snapshots where output is stable enough to lock down
  - targeted tests for hint selection on common parser failures
- [x] Validate the improved diagnostics against large real repositories
  - use real parsing/formatting failures encountered during late-stage validation
  - confirm the messages are understandable without reading the source code
  - trim noisy debug output until the signal is high enough for everyday use

### Acceptance criteria

- [x] Common parser/config/CLI failures produce actionable messages without requiring source-code inspection
- [x] A user can identify the failing file, line, and likely cause directly from CI logs
- [x] `--debug` output is sufficient to diagnose real formatting issues on representative repositories
- [x] Layout-related surprises can be traced to a concrete command-spec/config decision path
- [x] Diagnostics regressions are covered by snapshot or focused regression tests
- [x] The docs explain how to collect and share the right debugging information when reporting a bug

---

## Phase 12 — Final Tweaks

**Goal**: Validate late-stage operational behavior before release.

### Tasks

- [ ] Survey `cmakefmt --parallel` on very large codebases for RAM and system impact
      before ever considering a change from the default single-threaded execution
      model to a CPU-count default
- [ ] Use `oomph-lib` as a late-stage large-repository validation target for this survey
      - local checkout: `/Users/PuneetMatharu/Dropbox/programming/oomph-lib/oomph-lib-repos/forked-oomph-lib`
      - upstream repo: `https://github.com/oomph-lib/oomph-lib`
      - do not commit any `oomph-lib` CMake files or derived fixtures into this repository
      - keep this repository-specific validation scoped to Phase 12 only

---

## Phase 13 — Cross-Platform Support

**Goal**: Restore Windows as a first-class supported platform before the first public alpha.

### Tasks

- [x] Make `cmakefmt` work correctly on Windows developer machines and CI
- [x] Run the full test + clippy + fmt workflow on Windows in GitHub Actions
- [x] Audit and fix Windows-specific issues
  - path handling
  - newline handling
  - terminal colour handling
  - file discovery edge cases
  - in-place formatting behavior
  - docs/examples that assume POSIX shells

### Acceptance criteria

- [x] CI passes on Linux, macOS, and Windows
- [x] Core CLI flows behave correctly on Windows

---

## Phase 14 — Workflow Advantage

**Goal**: Make `cmakefmt` materially more useful in real developer workflows than `cmake-format`,
not just faster.

### Tasks

- [x] Add ignore-file support for recursive discovery
  - support `.cmakefmtignore`
  - optionally honor `.gitignore` by default for discovered files
  - allow one or more explicit `--ignore-path <PATH>` files
  - document precedence between explicit paths, ignore files, and direct file arguments
- [x] Add partial / range formatting for editor integrations
  - support line-range formatting on a single file (for example `--lines 10:25`)
  - support multiple ranges in one invocation
  - document that multi-line statements may force formatting slightly outside the selected range
  - keep range formatting unavailable in config files and scoped to explicit editor/CLI use
- [x] Add stdin virtual-path support
  - support `--stdin-path <PATH>` (or equivalent naming) so stdin formatting can use:
    - the right file type / extension semantics
    - the nearest project config for the virtual path
    - ignore-file decisions and debug output tied to a meaningful path
  - make editor integrations first-class rather than a special-case stdin hack
- [x] Add diff and machine-readable change output
  - add `--diff` to emit a unified diff instead of full formatted output
  - add a machine-readable replacement/report mode suitable for editor plugins and CI tooling
  - keep human-friendly stdout as the default, but make integration-friendly output stable
  - document output-format guarantees clearly enough for downstream tooling
- [x] Add VCS-aware file selection modes
  - support `--staged` for pre-commit / pre-push workflows
  - support `--changed` and `--since <REF>` for CI and incremental formatting
  - keep these modes opt-in and independent of recursive discovery defaults
  - document exact Git assumptions and failure modes clearly
- [x] Add file-list input modes for large scripted workflows
  - support `--files-from <PATH>` and `--files-from -`
  - accept newline-delimited and/or NUL-delimited path lists
  - make this compose cleanly with `--check`, `--diff`, and `--in-place`
- [x] Tighten editor and automation integration
  - document editor integration patterns for stdin + virtual path + range formatting
  - add CI examples using `--changed` / `--since`
  - add pre-commit examples using `--staged`
  - ensure every workflow feature has CLI regression coverage
- [x] Benchmark and validate the new workflow features
  - benchmark ignore-heavy directory walks
  - benchmark VCS-aware changed/staged modes on realistic repos
  - benchmark diff/range/stdin-path overhead
  - add real regression tests for ignore handling, diff output, range formatting, and VCS selection

### Acceptance criteria

- [x] `cmakefmt` can skip ignored files during recursive discovery without custom wrapper scripts
- [x] An editor can format stdin reliably using a virtual file path and, where supported, explicit line ranges
- [x] CI and pre-commit workflows can format only changed/staged files with first-party CLI support
- [x] `--diff` and the machine-readable output mode are stable enough for downstream tooling
- [x] Workflow-feature regressions are covered by CLI/integration tests and benchmarked for overhead

---

## Phase 15 — Alpha Release

**Goal**: Publish the first public alpha, automate repeatable releases, and make
`cmakefmt` easy to install and adopt across CLI, CI, and editor workflows.

### Tasks

- [ ] Define the alpha release contract
  - version `1.0.0-alpha.1`
  - supported platforms: Linux, macOS, Windows
  - supported CPU targets at minimum: `x86_64`; add `aarch64` where practical
  - clear statement of what "alpha" means: feature-complete enough for early adopters,
    but still open to formatting changes before `1.0`
- [ ] Create a repeatable release checklist
  - changelog/release notes process
  - tag naming convention
  - version bump process
  - verification steps before publish
  - rollback/yank procedure if a bad alpha ships
- [ ] Make GitHub Releases the binary distribution source of truth
  - build release binaries for:
    - Linux `x86_64`
    - Linux `aarch64`
    - macOS `x86_64`
    - macOS `aarch64`
    - Windows `x86_64`
  - publish `.tar.gz` archives for Unix platforms and `.zip` for Windows
  - attach `SHA256SUMS`
  - include install examples in release notes
- [ ] Automate the release pipeline in GitHub Actions
  - run full CI before publish
  - build all release artifacts from a tag
  - run smoke tests against built binaries
  - publish to crates.io
  - create/update the GitHub Release
  - fan out follow-up publishing jobs for package managers where credentials/automation allow
- [ ] Publish `cmakefmt` on crates.io
  - ensure `Cargo.toml` metadata is complete
  - include README, license, repository, keywords, categories, homepage/docs links
  - verify `cargo install cmakefmt` works from crates.io
- [ ] Finalize the project license before the first public alpha
  - choose the final SPDX expression
  - add or update the canonical `LICENSE*` files in the repo root
  - make Cargo metadata, README/docs, and release packaging agree on the license
  - document any third-party notice requirements that need to ship with release artifacts
- [ ] Use `reuse` to make copyright and license metadata explicit across the repo
  - decide which file types should carry SPDX/copyright headers directly
  - add `reuse` metadata/exceptions for generated files, snapshots, fixtures, and other non-header cases
  - run `reuse lint` and keep it in the release checklist
- [ ] Reduce the checked-in third-party fixture/license surface before alpha
  - stop shipping copied upstream real-world fixtures directly in the main repository if we can avoid it
  - replace them with fetch-at-test-time tooling and/or a separately versioned fixture archive
  - pin upstream sources/commits so real-world validation remains deterministic and reviewable
  - keep CI/local test runs practical, including a clear cache/offline story for contributors
  - ensure the repo root only needs license texts for project-owned files and intentionally bundled assets
- [ ] Publish first-party installation channels that we should maintain directly
  - GitHub Releases downloadable binaries
  - crates.io (`cargo install`)
  - Homebrew tap formula for macOS/Linux
  - `winget` manifest for Windows
  - Scoop bucket/package for Windows
  - npm wrapper package for JavaScript-heavy repos and CI environments
  - container image published to GHCR for CI and hermetic usage
- [ ] Prepare and publish additional package-manager channels where feasible during alpha
  - Chocolatey package
  - Arch Linux AUR package
  - Nix package / flake / `nix run` support
  - Debian/Ubuntu `.deb` package
  - Fedora/RHEL `.rpm` package or COPR repo
  - Alpine `apk` package if maintenance cost is acceptable
  - MacPorts port
  - `asdf` / `mise` plugin for version-manager installs
- [ ] Document channel ownership and support level
  - "officially maintained by this repo"
  - "automated but best-effort"
  - "community-maintained but linked from docs"
  - define which channels are blockers for alpha vs stretch goals during the alpha window
- [ ] Write installation and upgrade documentation
  - installation matrix by OS/package manager
  - copy-paste install commands
  - upgrade/uninstall commands
  - how to pin an alpha version in CI
  - how to verify checksums for downloaded binaries
- [ ] Provide copy-paste CI integration examples
  - GitHub Actions
  - GitLab CI
  - Azure Pipelines
  - generic shell/Docker examples
- [ ] Create an official GitHub Action for `cmakefmt`
  - support `check` mode for pull requests
  - support optional in-place formatting for bot/workflow usage
  - support version pinning
  - support file/path filters
  - document usage in the main README and in the action README
- [ ] Create a VS Code extension as an adoption accelerator
  - provide "format document" for `CMakeLists.txt` and `*.cmake`
  - use a bundled or downloaded `cmakefmt` binary
  - expose basic settings such as binary path, args, and config file
  - work on macOS, Linux, and Windows
  - publish to the VS Code Marketplace and Open VSX if practical
- [ ] Evaluate additional workflow features that would make `cmakefmt`
      meaningfully stronger than `cmake-format` before alpha
  - lower-noise automation modes:
    - `--quiet`
    - optional summary-only output for CI and pre-commit use
  - multi-file failure handling:
    - `--keep-going` / continue-on-error batch mode
    - end-of-run aggregated error summaries instead of fail-fast only
  - config introspection and explanation:
    - `--show-config`
    - `--show-config-path`
    - `--explain-config <PATH>`
  - config-discovery shortcuts for tooling:
    - `--find-config-path`
    - `--no-config`
    - possibly a `--config-precedence` mode for editor integrations
  - incremental cache support for repeated large-repo runs:
    - `--cache`
    - `--cache-location`
    - `--cache-strategy` (`metadata` vs `content`)
  - formatter safety / confidence modes:
    - `--verify` or `--safe` to reject output if the parse tree changes
    - `--fast` to skip that verification when users explicitly want speed
  - version pinning support:
    - `--required-version`
    - optional config-file equivalent so teams can lock formatter behavior
  - editor-grade machine output:
    - output replacements instead of full-file text
    - cursor-position translation for editor integrations
    - explicit byte-range formatting in addition to line ranges if it proves simpler for editors
  - richer CI/reporting formats:
    - `--reporter github`
    - `--reporter checkstyle`
    - `--reporter junit`
    - `--reporter sarif`
  - human-oriented batch summaries:
    - scanned / selected / changed / unchanged / skipped / failed counts
    - final summary output tailored for check-mode and in-place runs
  - gradual-adoption controls for large legacy repos:
    - opt-in format pragma support (`--require-pragma` style behavior)
    - possibly pragma insertion for migration tooling
  - skip visibility and discovery explanation:
    - explain why files were skipped by `.cmakefmtignore`
    - explain why files were skipped by `.gitignore`
    - explain why files were skipped by `--path-regex`, `--changed`, or `--staged`
    - make these reasons visible in `--debug` and/or dedicated explain modes
  - benchmark governance and regression tracking:
    - define how benchmark baselines are stored and compared from release to release
    - add benchmark CI or scheduled benchmark runs with artifact retention
    - define a regression review policy before alpha
  - formatting stability policy:
    - define what level of formatting churn is acceptable across alpha releases
    - document when a change is "style bug fix" vs "style break"
    - publish rollout advice for teams pinning formatter versions
- [ ] Ensure release-quality polish around distribution
  - `--version` reports the expected semver and commit/tag metadata where appropriate
  - shell completions are generated and shipped in release artifacts
  - man page / CLI reference is generated if we decide to maintain one
  - licenses for bundled artifacts/extensions/actions are correct
  - release docs explain how user config discovery works in packaged installs
  - release docs explain formatter stability expectations across alpha versions

### Acceptance criteria

- [ ] `cargo publish --dry-run` succeeds
- [ ] `reuse lint` succeeds and the repo has a consistent final license story
- [ ] Real-world validation no longer requires checked-in copied upstream fixture trees in the main repo
- [ ] Tagging `v1.0.0-alpha.1` from a clean commit can produce a complete release candidate without manual file editing
- [ ] GitHub Releases contains working binaries for Linux, macOS, and Windows, plus checksums
- [ ] `cargo install cmakefmt --version 1.0.0-alpha.1` works
- [ ] Homebrew install works from a clean machine
- [ ] Windows install works via both `winget` and Scoop
- [ ] At least one Linux-native package-manager path is available in addition to crates.io
  - preferred: Nix and/or AUR during alpha
- [ ] Container image can run `cmakefmt --check` against a mounted repository
- [ ] Official GitHub Action can format/check a repository in CI
- [ ] VS Code extension can format a real `CMakeLists.txt` using the released binary
- [ ] Installation docs cover every published channel and clearly label support level
- [ ] A user with no Rust toolchain can install and run `cmakefmt` on each supported OS

---

## Future (post-1.0)

- Revisit whether default parallelism should remain opt-in after very large
  codebase surveys
- LSP server mode (long-term)
- Additional editor plugins (Neovim, JetBrains, Helix) beyond the Phase 14 VS Code extension
- Linting rules (separate `cmake-lint` subcommand or binary)
