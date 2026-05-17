# Changelog

This project follows a simple changelog discipline:

- keep user-visible changes in `Unreleased` until the next cut
- group entries by impact, not by file
- call out migration-impacting changes explicitly

## Unreleased

## 1.5.0 — 2026-05-17

### Added

- New `cmakefmt manpage` subcommand that prints the roff man page
  to stdout, replacing the older `--generate-man-page` flag at
  the top-level CLI surface. The flag remains accepted as a
  hidden deprecated alias so existing release scripts and
  distribution recipes keep working during the transition;
  packagers should migrate to `cmakefmt manpage > cmakefmt.1`
  when convenient.

### Changed

- `--fast` is now a hidden alias of `--no-verify` instead of a
  visible one. Both flags continue to work; only the canonical
  `--no-verify` is documented in `--help` output and the CLI
  reference. Matches the "deprecated alias" framing the docs
  have used since v1.3.x.
- Annotated several public spec and config-enum types with
  `#[non_exhaustive]` so future field/variant additions stay
  patch-safe rather than breaking downstream consumers using
  exhaustive struct-literal construction or `match` arms:
  `KwargSpec`, `CommandForm`, `LayoutOverrides`, `NArgs`,
  `CommandSpec`, `CaseStyle`, `LineEnding`, `FractionalTabPolicy`,
  `ContinuationAlign`, `DangleAlign`, plus per-variant on
  `Error::IoAt` and `Error::LayoutTooWide`. The `Config`,
  `PerCommandConfig`, and AST node types (`CommandInvocation`,
  `BracketArgument`, `File`, `Statement`, `Argument`, `Comment`)
  intentionally stay un-annotated for now because external
  callers (tests, benchmarks, downstream tools) construct them
  via struct literals; their hardening is tracked on the roadmap
  for a later release that pairs the annotation with a builder
  API.

### Fixed

- The winget submission workflow (`publish-winget.yml`) now
  actually fires on each tagged release. Previously the
  `release.yml` job that publishes the GitHub Release used the
  default `GITHUB_TOKEN`, and GitHub's recursion guard prevents
  events emitted by `GITHUB_TOKEN` from triggering downstream
  workflows — so every release since `v0.8.0` published a
  GitHub Release without ever firing the winget submission.
  Confirmed against the workflow history: 10+ releases
  published, exactly one `publish-winget.yml` run, which was a
  manual `workflow_dispatch` from 2026-04-26 against `v1.3.0`.
  `release.yml` now passes `RELEASE_WORKFLOW_TOKEN` to the
  `softprops/action-gh-release` step, so the release-publish
  event comes from a PAT and downstream workflows trigger as
  intended. The token's scope requirements are documented in
  `RELEASING.md`.
- Files that begin with a UTF-8 byte-order mark (commonly
  written by MSVC / Visual Studio on Windows) now round-trip
  through the formatter with the BOM preserved. Previously the
  parser stripped the BOM during parsing and the formatter never
  re-prepended it, so every format pass silently changed the
  file's leading bytes — invisible to most editors but enough
  to trip strict file-content checks or break a build's
  encoding-detection step. Files without a BOM are unaffected
  (no BOM is ever added).
- `set_package_properties(PACKAGE … PROPERTIES <k> <v> …)` is now
  exempt from the autosort heuristic, completing the v1.4.2 sweep
  across the property-family commands. The kwarg has the same
  `<key> <value>` pair structure as `set_target_properties`; the
  v1.4.2 fix missed it because it lives in the `FeatureSummary`
  module spec rather than the core property commands. As before,
  the previous behaviour produced syntactically valid CMake with
  corrupted semantics.

### Internal

- Removed a dead conditional in `formatter::node::format_section_inline`
  whose two arms returned identical values; the `header_kind`
  parameter became unused and was dropped from the signature.
  No behaviour change.

### Documentation

- The Formatting Cookbook no longer recommends the renamed-away
  config keys `use_tabchars` and `separate_ctrl_name_with_space`;
  these are accepted as legacy aliases but the current spellings
  are `use_tabs` and `space_before_control_paren`. The recipe link
  to the custom-commands section of the config reference also got
  a corrected anchor (`#custom-command-specs`).
- The FAQ no longer references a nonexistent `--command-spec` CLI
  flag (custom commands live in the `commands:` section of the
  config file); the editor-integration link now points to the
  correct `/editors/` page; the FAQ is now reachable from the
  sidebar under Reference.
- The Playground page now includes a short description of the
  source presets, config presets, shareable URLs, and auto-refresh
  behaviour, so the new feature surface is discoverable from
  outside the component itself.
- Bumped pinned-version examples in the CI guide from `1.3.0` to
  `1.4.2`, and updated the JSON-LD `softwareVersion` blob on the
  homepage to match the latest release.
- Troubleshooting guide now references `--no-verify` instead of
  the deprecated `--fast` alias.

## 1.4.2 — 2026-05-16

### Fixed

- `set_property(... PROPERTY <name> <values…>)` and the
  `set_target_properties` / `set_source_files_properties` /
  `set_directory_properties` / `set_tests_properties` commands are now
  exempt from the autosort heuristic. Previously, with `autosort`
  enabled, the formatter would flat-sort the tokens inside `PROPERTY`
  and `PROPERTIES` value lists — silently moving the property name out
  of its positional slot in `set_property`, and scrambling `<key>
  <value>` pairs in the `*_properties` family. The reordered output
  was still valid CMake syntax but changed the command's semantics.
  These commands now round-trip unchanged regardless of `autosort`.
- Decorative banner comments composed entirely of `#` characters
  (e.g. `########################################`) are preserved
  verbatim instead of being collapsed to a single `#` line by the
  comment reflow logic.

### Documentation

- The browser playground now includes source presets for common formatter
  scenarios, config presets that keep the full default configuration visible,
  shareable URLs for the current source/config state, and automatic output
  refresh when switching presets.
- The homepage formatter animation now uses `astro-magic-move`'s public
  `step` / `totalSteps` API, so the dependency can track upstream releases
  without relying on private component internals.

## 1.4.1 — 2026-05-14

### Fixed

- Linux wheels now install on glibc ≥ 2.28 (RHEL 8/9, Ubuntu 22.04+,
  Debian 11+, SLES 15) instead of requiring glibc ≥ 2.39, which had
  excluded every in-support enterprise distro. Wheels are now built
  inside a `manylinux_2_28` container. (#47)
- Lowered `requires-python` from `>=3.11` to `>=3.8`. The published
  wheel is a standalone Rust binary with no Python runtime
  dependency, so the previous floor was preventing installs on
  systems whose Python happened to be older even though nothing
  about the package needed 3.11. (#47)

## 1.4.0 — 2026-05-02

### Added

- Structurally-correct command specs for ~50 commonly-used CMake
  module commands — the commands defined in CMake-bundled modules
  that become available after `include(<Module>)` or
  `find_package(<Module>)`. These previously formatted as flat
  positional lists; the formatter now recognises kwargs and flags
  for layout, autosort eligibility, and pair-aware rendering.
  Coverage spans:
  - **FetchContent**: `FetchContent_Declare` (with the full source-fetch
    surface plus `EXCLUDE_FROM_ALL` / `SYSTEM` /
    `OVERRIDE_FIND_PACKAGE` flags and `FIND_PACKAGE_ARGS`),
    `FetchContent_MakeAvailable`, `FetchContent_GetProperties`,
    `FetchContent_Populate`, `FetchContent_SetPopulated` (corrected
    from a 3-positional misspelling to the documented kwarg form).
  - **ExternalProject**: `ExternalProject_Add` with all ~90 kwargs
    across the source-fetch, configure, build, install, and test
    phases — the largest single command spec in the project.
    `ExternalProject_Add_Step`, `ExternalProject_Add_StepTargets`,
    `ExternalProject_Add_StepDependencies`,
    `ExternalProject_Get_Property`.
  - **Check\* family**: `check_language`, `check_compiler_flag` and
    its per-language variants, `check_include_file*`,
    `check_function_exists`, `check_library_exists`,
    `check_symbol_exists`, `check_cxx_symbol_exists`,
    `check_type_size`, `check_variable_exists`, the
    `check_<lang>_source_compiles`/`_runs` family,
    `check_source_compiles`/`_runs`, `check_linker_flag`,
    `check_prototype_definition`, `check_struct_has_member`,
    `check_ipo_supported`, `check_pie_supported`,
    `check_cxx_accepts_flag`.
  - **CMakePushCheckState**: `cmake_push_check_state` (with `RESET`
    flag), `cmake_pop_check_state`, `cmake_reset_check_state`.
  - **CMakePackageConfigHelpers**: `configure_package_config_file`
    (with `INSTALL_DESTINATION` / `PATH_VARS` / `INSTALL_PREFIX` and
    `NO_SET_AND_CHECK_MACRO` / `NO_CHECK_REQUIRED_COMPONENTS_MACRO`
    flags), `write_basic_package_version_file`,
    `write_basic_config_version_file`.
  - **GoogleTest**: `gtest_add_tests`, `gtest_discover_tests` (with
    the full `EXTRA_ARGS` / `WORKING_DIRECTORY` / `TEST_PREFIX` /
    `PROPERTIES` / `DISCOVERY_TIMEOUT` / `DISCOVERY_MODE` surface).
  - **GenerateExportHeader**: `generate_export_header` (with all 10
    macro-naming kwargs and `DEFINE_NO_DEPRECATED` flag).
  - **CMakeFindDependencyMacro**: `find_dependency` (full forward
    of `find_package`'s flag and kwarg surface).
  - **FindPackageHandleStandardArgs**:
    `find_package_handle_standard_args`,
    `find_package_check_version`, `find_package_message`.
  - **FindPkgConfig**: `pkg_check_modules`, `pkg_search_module`,
    `pkg_get_variable`.
  - **CMakeParseArguments**: `cmake_parse_arguments` (still
    available alongside the builtin since 3.7).
  - **CMakePrintHelpers**: `cmake_print_properties`,
    `cmake_print_variables`.
  - **CPackComponent**: `cpack_add_component`,
    `cpack_add_component_group`, `cpack_add_install_type`,
    `cpack_configure_downloads`.
  - **CPackIFW**: `cpack_ifw_configure_component`,
    `cpack_ifw_configure_component_group`, `cpack_ifw_add_repository`,
    `cpack_ifw_update_repository`, `cpack_ifw_add_package_resources`.
  - **CPackIFWConfigureFile**: `cpack_ifw_configure_file`.
  - **BundleUtilities**: `fixup_bundle`, `verify_app`.
  - **AndroidTestUtilities**: `android_add_test_data`.
  - **Other**: `test_big_endian`, `write_compiler_detection_header`,
    `cmake_dependent_option`, `processorcount`,
    `select_library_configurations`.

### Removed

- The `--preview` CLI flag and the `[experimental]` config section
  (and its underlying `Experimental` struct) have been dropped. Both
  were no-ops on every release that exposed them — the formatter has
  no preview-gated behaviour at the moment, so passing the flag or
  populating the section did nothing. Removing them keeps the public
  surface honest. If a future release introduces opt-in preview
  behaviour, the gate will return; until then there is nothing for it
  to gate.
- The `[markup] explicit_trailing_pattern` config option has been
  removed. It was wired into the config schema but never consulted by
  the formatter — no path read the user-supplied regex, so a custom
  pattern produced no observable effect. Removing the dead surface
  prevents config drift where users would set it and silently get the
  default behaviour.

### Fixed

- The Language Server Protocol entry points
  (`textDocument/formatting`, `textDocument/rangeFormatting`,
  `textDocument/codeAction`) now return proper error responses for
  malformed requests instead of silently dropping them. Failed
  parameter extraction surfaces as `InvalidParams`; failed JSON
  serialisation surfaces as `InternalError`. Editors and LSP test
  harnesses that wait for a response no longer hang on these paths.
- A user-supplied command override declaring a `Discriminated` spec
  with an empty `forms` map and no `fallback` no longer crashes the
  formatter. The lookup now degrades to a static empty
  `CommandForm` rather than panicking via `.expect()`.

### Changed

- Filesystem error messages now include the offending path. Failures
  reading config files, source files, the cache, install-hook
  destinations, or atomic-write tempfiles render as
  `error: I/O failure reading <path>: <reason>` instead of a bare
  `<reason>`. Streaming I/O (stdin/stdout) is unaffected and continues
  to surface its native error.
- The embedded command spec is now split across two YAML sources:
  `src/spec/builtins.yaml` (commands listed by
  `cmake --help-command-list`) and `src/spec/modules.yaml` (commands
  defined in CMake-bundled modules). The runtime decodes both
  MessagePack blobs and merges them into a single command table at
  startup; spec consumers see no difference. The split mirrors the
  natural taxonomy users already use to think about CMake commands
  and keeps `builtins.yaml` focused on the language surface. Module
  commands previously specced in `builtins.yaml` (the Check\* family
  and friends) have been migrated verbatim to `modules.yaml`.

### Distribution

- Automated winget submissions on each release. A new
  `publish-winget.yml` workflow fires on `release: published` and
  opens a PR against `microsoft/winget-pkgs` with the new manifest,
  using a shared fork at `cmakefmt/winget-pkgs`. Manual retries are
  available via `workflow_dispatch`. The first run closed the
  pre-existing manifest gap (winget had been stuck at 0.3.0); from
  v1.3.0 onward, `winget upgrade` tracks releases on the same
  cadence as Homebrew.

### Documentation

- Overhauled the GitHub README. Trimmed from 356 lines to 130 by
  removing content owned by the docs site (Common Workflows table,
  Configuration section, Formatter Disable Regions, Library Usage
  example, full performance fixture table, Project Layout, Development
  commands). Added a focused GitHub Action section showcasing the new
  `mode`/`scope` inputs. Each remaining sentence is a complete
  thought rather than a colon-and-link fragment.
- Rewrote the CI Integration page to use the new `cmakefmt-action`
  surface. The `mode` input (`check`/`diff`/`fix`/`setup`) and `scope`
  input (`all`/`changed`/`staged`) replace the older `args:`-based
  examples throughout, with `paths`, `since`, `working-directory`,
  and `version` covering the rest. Added a changed-file rollout
  example, a monorepo example, and an auto-format-and-commit
  example. Bumped the Docker tag pin from 0.4.0 to 1.3.0 to match
  the current release.
- Tightened the winget messaging in the installation page. With
  manifest submission now automated, the "version updates may lag
  releases slightly" hedge has been removed and the support-levels
  table promotes winget from "Community maintained" to "Officially
  maintained".

### Performance

- Single-file wall time on the 656-line `mariadb_server` fixture
  holds at 6.0 ms — unchanged from v1.3.0 despite the spec growing
  significantly with the new module command coverage. Build-time
  MessagePack pre-deserialisation (introduced in v1.2.0) continues
  to absorb the parse cost; the larger lookup table and MessagePack
  blob are decoded at startup with no measurable difference.
  Release binary size is unchanged at 4.7 MB. Methodology unchanged
  from v1.2.0: `hyperfine --shell=none --style basic --warmup 100
  --runs 200`.

## 1.3.0 — 2026-04-25

### Added

- Structurally-correct command specs for ~60 previously-stubbed CMake
  builtins. Each spec follows the canonical `cmake --help-command`
  synopsis and gives the formatter keyword/flag awareness for layout
  decisions (inline packing, vertical wrapping, autosort eligibility,
  pair-aware rendering). Coverage spans:
  - **Trivial commands**: `break`, `continue`, `enable_testing`,
    `aux_source_directory`, `mark_as_advanced`, `add_compile_definitions`,
    `add_compile_options`, `add_definitions`, `add_dependencies`,
    `add_link_options`, `link_directories`, `link_libraries`,
    `include_directories`, `include_regular_expression`, `site_name`,
    `get_cmake_property`, `enable_language`.
  - **Single-form commands with kwargs/flags**: `build_command`,
    `define_property`, `set_directory_properties`, `set_tests_properties`,
    `set_source_files_properties`, `target_compile_features`,
    `variable_watch`, `fltk_wrap_ui`, `qt_wrap_cpp`, `qt_wrap_ui`,
    `include_external_msproject`, `create_test_sourcelist`,
    `separate_arguments`, `cmake_host_system_information`,
    `cmake_file_api`, `get_target_property`, `get_test_property`,
    `get_source_file_property`.
  - **CTest family** (full kwarg coverage on every `ctest_*` builtin):
    `ctest_build`, `ctest_configure`, `ctest_coverage`,
    `ctest_empty_binary_directory`, `ctest_memcheck`,
    `ctest_read_custom_files`, `ctest_run_script`, `ctest_sleep`,
    `ctest_start`, `ctest_submit`, `ctest_test`, `ctest_update`,
    `ctest_upload`.
  - **Multi-form discriminated commands**: `add_test` (NAME-form vs
    legacy positional fallback), `cmake_policy` (VERSION/SET/GET/PUSH/POP),
    `source_group` (TREE-form vs default fallback), `cmake_path` (full
    coverage of all 30+ subcommands: GET/HAS_*/IS_*/COMPARE/SET/APPEND/
    APPEND_STRING/REMOVE_FILENAME/REPLACE_FILENAME/REMOVE_EXTENSION/
    REPLACE_EXTENSION/NORMAL_PATH/RELATIVE_PATH/ABSOLUTE_PATH/
    NATIVE_PATH/CONVERT/HASH).
  - **Approximations of scope-discriminated commands**:
    `get_directory_property`, `get_property`, `set_property`,
    `get_filename_component`, `load_cache`, `try_compile`, `try_run`
    — modelled as single-form with combined kwargs/flags rather than
    full multi-scope discrimination, which is sufficient for current
    layout decisions but may be promoted to true `forms:` later.

### Fixed

- Discriminated command fallback dispatch. `add_test` and `source_group`
  previously declared their non-discriminator signatures under a
  literally-named `DEFAULT` form, but `form_for()` looks up forms by
  exact first-arg match. When the first arg wasn't `DEFAULT` the
  lookup fell through to the first form in insertion order rather
  than the intended catch-all. For
  `source_group("Source Files" FILES … REGULAR_EXPRESSION …)` this
  meant the TREE form was applied to a non-TREE call, so
  `REGULAR_EXPRESSION` was not recognised as a kwarg and got
  swallowed into the `FILES` value list. The catch-all now lives
  under the sibling `fallback:` field, which `form_for()` consults
  via `.or(fallback.as_ref())`. Both commands round-trip correctly.

### Documentation

- Annotated the 16 deprecated CMake commands that remain in
  `builtins.yaml` as `pargs: "*"` stubs (e.g. `install_files`,
  `install_programs`, `install_targets`, `subdirs`, `make_directory`,
  `exec_program`, `output_required_files`, `remove`,
  `use_mangled_mesa`, `utility_source`, `variable_requires`,
  `write_file`, `load_command`, `export_library_dependencies`,
  `subdir_depends`, `build_name`) with `# Deprecated since CMake X.Y`
  YAML comments pointing at their modern replacements. The gap is
  now explicit, not an oversight.

### Internal

- 12 new snapshot tests for the new specs, each using a narrow
  `line_width` (40 or 50) to force wrapping and assert structural
  separation: flags rendered on their own lines, kwargs as separate
  keyword sections, multi-form fallback dispatch wired correctly.
  Round-tripping inline-only would have masked the fallback bug
  above.

### Performance

- Single-file wall time on the 656-line `mariadb_server` fixture
  drops from 6.6 ms (v1.2.0) to 6.0 ms — a 9% improvement despite
  the spec growing significantly with the Phase 47g coverage
  additions. Build-time MessagePack pre-deserialisation
  (introduced in v1.2.0) absorbs the larger spec at zero runtime
  cost; the speedup comes from incidental optimisations in the
  splitter and layout paths exercised by the new structural
  awareness. Methodology unchanged from v1.2.0:
  `hyperfine --shell=none --style basic --warmup 100 --runs 200`.
  Release binary size unchanged at 4.7 MB.

## 1.2.0 — 2026-04-25

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

### Performance

- Faster startup. The embedded built-in command spec is now
  pre-deserialised at build time (`build.rs` reads the YAML source
  and emits a MessagePack blob into `OUT_DIR`); the runtime decodes
  that blob via `rmp-serde` instead of parsing structured text on
  every invocation. Single-file wall time on the 656-line
  mariadb_server fixture drops from 6.8 ms (v1.1.0) to 6.6 ms
  under matched-methodology hyperfine measurements
  (`--shell=none --style basic`, 100 warmups, 200 runs). The
  improvement holds even though the install() restructure grew
  the spec ~4×.

### Internal

- Migrated the embedded built-in command spec from
  `src/spec/builtins.toml` to `src/spec/builtins.yaml` for
  human-edit ergonomics, then pre-deserialises the YAML at build
  time (see Performance, above). The runtime no longer parses
  text at all; it decodes a MessagePack blob produced once during
  `cargo build`. User config and spec override files still accept
  both TOML and YAML — only the embedded baseline path changed.

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
