# Roadmap

Each phase has clear acceptance criteria. Do not move to the next phase
until all criteria for the current phase are met.

---

## Phase 1 — Scaffold + Grammar

**Goal**: A Rust project that can parse any valid CMake file without crashing.

### Tasks

- [x] `cargo new cmfmt --lib` + binary target in `Cargo.toml`
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
  - `cmfmt [FILE]...` — format and print to stdout
  - `cmfmt` — recursively find CMake files under the current working directory
  - `cmfmt -i [FILE]...` — format in-place
  - `cmfmt --check [FILE]...` — exit 1 if any file would change
  - `cmfmt --list-files [FILE|DIR]...` / `cmfmt --dry-run ...` — list files that would be reformatted
  - `cmfmt -f, --file-regex <REGEX>` — filter recursively discovered CMake files
  - `cmfmt -` — read from stdin, write to stdout
  - `cmfmt --dump-config` — print a default config template
  - `cmfmt --config <PATH>` — explicit config file
  - `cmfmt --line-width <N>` — override config
  - `cmfmt --version` — print version
- [x] Correct exit codes (0 = ok, 1 = check failed, 2 = error)
- [x] Helpful error messages (file path + line:col for parse errors)
- [x] Integration tests for CLI behaviour

### Acceptance criteria

- [x] `cmfmt --check` in CI workflow on a correctly formatted file returns 0
- [x] `cmfmt --check` on an unformatted file returns 1 (not 2)
- [x] `-i` modifies file and leaves it idempotent
- [x] Formatting 100 files in one invocation works correctly

---

## Phase 7 — Real-World Validation

**Goal**: The formatter produces reasonable output on real CMake projects.

### Tasks

- [ ] Collect 10+ real-world `CMakeLists.txt` files from popular projects
  (CMake, LLVM, Qt, OpenCV, Boost, abseil, googletest, etc.)
- [ ] Add them to `tests/fixtures/real_world/`
- [ ] Run formatter on all of them; manually review output
- [ ] Fix any formatting regressions found
- [ ] Snapshot all real-world outputs

### Acceptance criteria

- [ ] All real-world files pass idempotency
- [ ] Formatted output is reviewed and judged reasonable by a human
- [ ] No panics or errors on any real-world input

---

## Phase 8 — Performance & Polish

**Goal**: Fast, reliable, releasable.

### Tasks

- [ ] Add `criterion` benchmarks for parsing and formatting
- [ ] Profile with `cargo flamegraph` if benchmarks are unexpectedly slow
- [ ] Target: format a 1000-line `CMakeLists.txt` in < 10ms
- [ ] Add `cmfmt --debug` mode to print config resolution, file discovery matches,
      parser locations, and formatter layout decisions for diagnostics
- [ ] **Head-to-head benchmark: `cmfmt` vs `cmake-format`**
  - Install `cmake-format` (`pip install cmakelang`) in the benchmark environment
  - Run both tools against every file in `tests/fixtures/real_world/`
  - Measure wall-clock time (hyperfine or criterion shell benchmark)
  - Report speedup factor for each file and overall geometric mean
  - Include results table in `README.md`
- [ ] Write a `README.md` with installation, usage, config reference
- [x] Add pre-commit hook config example (`.pre-commit-config.yaml`)
- [x] Add GitHub Actions CI (test + clippy + fmt on Linux/macOS/Windows)
- [ ] Version `1.0.0-alpha.1` release on crates.io

### Acceptance criteria

- [ ] Benchmark target met (< 10ms per 1000-line file)
- [ ] `cmfmt` is measurably faster than `cmake-format` on every real-world fixture
- [ ] CI passes on all three platforms
- [ ] `cargo publish --dry-run` succeeds

---

## Future (post-1.0)

- `--diff` mode: show unified diff of changes
- `--files-from <FILE>` mode: read list of files from stdin/file
- Parallel formatting with `rayon`
- LSP server mode (long-term)
- Editor plugins (VS Code, Neovim) using the formatter as a library
- Linting rules (separate `cmake-lint` subcommand or binary)
