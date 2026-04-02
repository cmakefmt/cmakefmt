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
- [ ] The following real-world files parse correctly:
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

**Goal**: Formatter behaviour is fully configurable via TOML.

### Tasks

- [x] Define `Config` struct with all options (see ARCHITECTURE.md)
- [x] Implement config file loading + directory-walk resolution
- [x] Implement CLI flag overrides
- [x] `command_case` option (lower/upper/unchanged)
- [x] `keyword_case` option (lower/upper/unchanged)
- [x] `line_length` option
- [x] `tab_size` option
- [x] `use_tabchars` option
- [x] `max_empty_lines` option
- [x] `dangle_parens` + `dangle_align` options
- [x] `separate_ctrl_name_with_space` option
- [x] `separate_fn_name_with_space` option
- [x] `max_lines_hwrap`, `max_pargs_hwrap`, `max_subgroups_hwrap` options
- [x] `min_prefix_chars`, `max_prefix_chars` options
- [x] `[markup]` section: `enable_markup`, `first_comment_is_literal`,
      `literal_comment_pattern`, `bullet_char`, `enum_char`,
      `fence_pattern`, `ruler_pattern`, `hashruler_min_length`,
      `canonicalize_hashrulers`
- [x] `[per_command.<name>]` overrides — let users tune any option per-command
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
  - `cmakefmt --list-files [FILE|DIR]...` / `cmakefmt --dry-run ...` — list files that would be reformatted
  - `cmakefmt -f, --file-regex <REGEX>` — filter recursively discovered CMake files
  - `cmakefmt -` — read from stdin, write to stdout
  - `cmakefmt --dump-config` — print a default config template
  - `cmakefmt --config <PATH>` — explicit config file
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
- [ ] Optimize the hottest paths aggressively but safely:
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
- [x] Add GitHub Actions CI (test + clippy + fmt on Linux/macOS/Windows)

### Acceptance criteria

- [x] Benchmark target met (< 10ms per 1000-line file)
- [x] `cmakefmt` is measurably faster than `cmake-format` on every real-world fixture
- [x] Benchmark methodology and reproduction commands are documented
- [x] At least one profiling pass was performed on the hottest workloads and acted on
- [x] Parallel mode speedup and memory impact are measured and recorded
- [ ] CI passes on all three platforms

---

## Phase 9 — Completeness

**Goal**: Fill out the supported command surface so the formatter behaves well across the full CMake ecosystem we target.

### Tasks

- [ ] Audit `src/spec/builtins.toml` to a full built-in and supported module-command surface,
      including the utility/deprecated modules we intend to recognize and format well

---

## Phase 10 — User Friendliness

**Goal**: Make the project easy to adopt, understand, and contribute to.

### Tasks

- [ ] Define the published documentation structure and navigation for GitHub Pages
  - landing page / project overview
  - installation page
  - CLI usage reference
  - configuration reference
  - formatter behavior guide
  - migration / compatibility guide
  - library API guide
  - architecture / implementation overview
  - changelog / release notes
- [ ] Expand the user-facing docs into a full guide set
  - installation from Cargo and supported package managers
  - quick-start workflow for local formatting, `--check`, and `-i`
  - file discovery, `--file-regex`, `--list-files`, and `--parallel`
  - barriers and fence regions
  - debug mode and troubleshooting
  - benchmark / performance guide
- [ ] Write a complete CLI reference
  - every flag with behavior, defaults, and exit-code semantics
  - examples for single-file, recursive, stdin, in-place, and CI usage
  - examples for config overrides and `--dump-config`
  - note which flags are operational-only versus config-backed
- [ ] Write a complete configuration reference
  - document every config section and option
  - show the built-in defaults
  - explain config resolution precedence
  - explain per-command overrides
  - explain which markup features are opt-in and what they do today
- [ ] Write a formatter behavior guide
  - blank-line preservation
  - comment preservation and comment reflow behavior
  - control-flow indentation rules
  - horizontal wrap versus vertical layout heuristics
  - barriers / disabled regions / fence passthrough
  - known formatting differences from `cmake-format`
- [ ] Write a migration and compatibility guide for `cmake-format` users
  - command-line invocation mapping
  - config-file compatibility and differences
  - unsupported or intentionally different options
  - output-style differences that users should expect
  - rollout advice for CI / pre-commit migration
- [ ] Write a library/API guide for embedders
  - public crate entry points
  - config construction and override flow
  - error model
  - examples for `format_source` and related helpers
  - expectations around stability before and after `1.0`
- [ ] Keep an explicit changelog and release-note process
  - choose the changelog format
  - document categories for user-visible changes
  - link releases to changelog entries
  - ensure breaking changes and migration notes are called out clearly
- [ ] Add directory-level contributor readmes explaining the repo structure
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
- [ ] Add doc maintenance rules so docs stay in sync with code
  - update `CONTRIBUTING.md` with doc update requirements where needed
  - identify which changes must also update `README.md`, docs pages, and changelog
  - add lightweight validation for docs where practical (link checks or doc build)
- [ ] Publish the docs site through GitHub Pages
  - choose the site generator / layout
  - add the build/deploy workflow
  - ensure local preview instructions exist
  - ensure versioned or release-tagged docs policy is clear

### Acceptance criteria

- [ ] A new user can install `cmakefmt`, format a project, and understand the major CLI/config features from the docs alone
- [ ] A `cmake-format` user can follow a migration guide and understand compatibility gaps before switching
- [ ] Every public CLI flag and config option is documented in exactly one primary reference page
- [ ] Contributor readmes exist for the main repo areas and explain where new code, fixtures, snapshots, and docs belong
- [ ] The GitHub Pages docs site builds cleanly and is linked from `README.md`
- [ ] User-visible changes have a documented changelog/release-note path

---

## Phase 11 — Final Tweaks

**Goal**: Validate late-stage operational behavior before release.

### Tasks

- [ ] Survey `cmakefmt --parallel` on very large codebases for RAM and system impact
      before ever considering a change from the default single-threaded execution
      model to a CPU-count default

---

## Phase 12 — Alpha Release

**Goal**: Publish the first alpha and make installation easy.

### Tasks

- [ ] Version `1.0.0-alpha.1` release on crates.io
- [ ] Publish `cmakefmt` to package managers as the final distribution step
  - Homebrew
  - Other relevant package managers

### Acceptance criteria

- [ ] `cargo publish --dry-run` succeeds

---

## Future (post-1.0)

- `--diff` mode: show unified diff of changes
- `--files-from <FILE>` mode: read list of files from stdin/file
- Revisit whether default parallelism should remain opt-in after very large
  codebase surveys
- LSP server mode (long-term)
- Editor plugins (VS Code, Neovim) using the formatter as a library
- Linting rules (separate `cmake-lint` subcommand or binary)
