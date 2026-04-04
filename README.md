# `cmakefmt`

`cmakefmt` is a Rust CMake formatter intended to replace [`cmake-format`](https://github.com/cheshirekow/cmake_format) with a
single fast binary.

## Status

The project currently supports:

- parsing real CMake syntax including bracket arguments, comments, variable
  references, and generator expressions
- formatting CMake files via CLI or library API
- preserving comments and blank lines
- respecting `cmake-format: off` / `cmake-format: on`,
  `cmakefmt: off` / `cmakefmt: on`, and `# ~~~` fence regions
- configuration via `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`
  with YAML preferred for user-edited configs
- a built-in command registry audited through CMake 4.3.1, including the
  supported utility/deprecated module-command surface

The formatter is still under active development. Large-codebase parallel
surveying, cross-platform release work, and distribution are not complete yet.

The command spec version and audit date are stored in
`src/spec/builtins.toml` under `[metadata]`.

## Documentation

Primary docs entry points:

- [Documentation Book Source](site/src/README.md)
- [Repository Docs Index](docs/README.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

The user docs are authored as an `mdBook` under `site/` and published to
GitHub Pages from the built book output.

## Build

```bash
cargo build
```

Run the CLI directly from the repo:

```bash
cargo run -- --help
```

## CLI usage

Format one or more files to stdout:

```bash
cmakefmt CMakeLists.txt cmake/CompilerWarnings.cmake
```

When stdout is a terminal, lines changed by formatting are highlighted in cyan
by default. Use `--colour never` to disable that, or `--colour always` to force
ANSI colour output.

Recursively discover CMake files from the current directory and print the
formatted output:

```bash
cmakefmt
```

Format in place:

```bash
cmakefmt -i CMakeLists.txt
```

Format files in parallel with two workers:

```bash
cmakefmt --parallel 2 -i .
```

Show a progress bar during an in-place run:

```bash
cmakefmt --progress-bar --in-place .
```

Use all available CPUs explicitly:

```bash
cmakefmt --parallel --check .
```

Check formatting in CI or pre-push hooks:

```bash
cmakefmt --check CMakeLists.txt
```

List the files that would be reformatted without modifying them:

```bash
cmakefmt --list-files
cmakefmt --list-files path/to/project
```

Restrict recursive discovery with a regex:

```bash
cmakefmt --list-files --path-regex 'modules|toolchain' .
```

Read from stdin:

```bash
cat CMakeLists.txt | cmakefmt -
```

Use an explicit config file:

```bash
cmakefmt --config-file path/to/.cmakefmt.yaml CMakeLists.txt
```

Merge multiple config files explicitly, with later files overriding earlier
ones:

```bash
cmakefmt --config-file base.yaml --config-file team.yaml CMakeLists.txt
```

Override config values on the command line:

```bash
cmakefmt --line-width 100 --tab-size 4 --command-case lower --keyword-case upper CMakeLists.txt
```

Print the default YAML config template:

```bash
cmakefmt --dump-config
```

Print the TOML config template explicitly:

```bash
cmakefmt --dump-config toml
```

Convert a legacy `cmake-format` config file:

```bash
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.toml
```

Print debug diagnostics while checking a file:

```bash
cmakefmt --debug --check CMakeLists.txt
```

If a parse or config failure occurs, `cmakefmt` now prints:

- the failing file path
- 1-based line and column
- a short source snippet with a caret
- a likely-cause hint where possible
- a copy-paste repro command using `--debug --check`

Current CLI flags:

```text
cmakefmt [OPTIONS] [FILES]...

  -i, --in-place
      --check
      --list-files
      --path-regex <REGEX>
      --dump-config
      --convert-legacy-config <PATH>
      --debug
      --colour <auto|always|never>
      --parallel [<JOBS>]
      --progress-bar
      --config-file <PATH>
      --line-width <N>
      --tab-size <N>
      --command-case <lower|upper|unchanged>
      --keyword-case <lower|upper|unchanged>
      --dangle-parens <true|false>
  -h, --help
  -V, --version
```

The clearer `cmakefmt` long-form flags are primary where applicable.
`--path-regex` replaces the older `--file-regex`. `--config` is still accepted
as an alias for `--config-file` to ease migration.

Exit codes:

- `0`: success
- `1`: `--check` or `--list-files` found files that would change
- `2`: parse, config, or I/O error

## Debugging And Bug Reports

When reporting a formatting or parsing issue, include:

- the failing file path
- the exact `cmakefmt` command you ran
- the full error output or `--debug` output
- the relevant `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` files if
  config is involved

The quickest reproducible capture is usually:

```bash
cmakefmt --debug --check path/to/CMakeLists.txt
```

## Formatter barriers

`cmakefmt` supports selectively disabling formatting for regions of a file.
These barrier lines are preserved verbatim:

```cmake
# cmakefmt: off
# cmakefmt: on
```

It also recognizes the existing `cmake-format` spellings:

```cmake
# cmake-format: off
# cmake-format: on
```

Fence regions are supported with `# ~~~`:

```cmake
# ~~~
# everything in here is passed through unchanged
# ~~~
```

Disabled regions are emitted unchanged, even if they would not parse as valid
CMake on their own.

## Performance

Current local benchmark signal:

- `format_source/large_synthetic` (1000+ lines): `8.6263 ms .. 8.8934 ms`
- `cmakefmt` is faster than `cmake-format 0.6.13` on every file in the current
  real-world corpus
- geometric-mean speedup across that corpus: `20.77x`
- `--parallel 8` improves whole-corpus throughput by `3.43x` over the default
  single-threaded mode on the current synthetic batch workload

Head-to-head real-world corpus results:

| Fixture                         | Lines | `cmakefmt` ms | `cmake-format` ms | Speedup |
|---------------------------------|------:|--------------:|------------------:|--------:|
| `abseil/CMakeLists.txt`         |   204 |         4.467 |           114.576 |  25.65x |
| `catch2/CMakeLists.txt`         |   231 |         4.558 |           101.606 |  22.29x |
| `cli11/CMakeLists.txt`          |   283 |         4.458 |           118.954 |  26.68x |
| `cmake_cmbzip2/CMakeLists.txt`  |    25 |         3.957 |            59.156 |  14.95x |
| `googletest/CMakeLists.txt`     |    36 |         4.138 |            60.558 |  14.64x |
| `llvm_tablegen/CMakeLists.txt`  |    83 |         4.257 |            73.627 |  17.30x |
| `monorepo_root.cmake`           |    40 |         4.330 |            69.929 |  16.15x |
| `nlohmann_json/CMakeLists.txt`  |   237 |         4.717 |           131.813 |  27.95x |
| `opencv_flann/CMakeLists.txt`   |     2 |         3.899 |            49.754 |  12.76x |
| `protobuf/CMakeLists.txt`       |   201 |         4.631 |            85.811 |  18.53x |
| `qtbase_network/CMakeLists.txt` |   420 |         5.557 |           279.420 |  50.28x |

The full methodology, profiler notes, and serial-versus-parallel memory
measurements live in
[docs/PERFORMANCE.md](docs/PERFORMANCE.md).

## Configuration

The formatter looks for config files in this order:

1. repeated `--config-file <PATH>` files, if provided
2. the nearest `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml` found by
   walking upward from the file
3. `~/.cmakefmt.yaml`, `~/.cmakefmt.yml`, or `~/.cmakefmt.toml`
4. built-in defaults

When more than one supported config filename exists in the same directory,
`cmakefmt` prefers YAML over TOML.

Example config:

```yaml
format:
  line_width: 100
  tab_size: 4
  use_tabs: false
  max_empty_lines: 1
  dangle_parens: true
  dangle_align: prefix
  space_before_control_paren: false
  space_before_definition_paren: false

style:
  command_case: lower
  keyword_case: upper

markup:
  enable_markup: true
  reflow_comments: true
  first_comment_is_literal: true

per_command:
  message:
    line_width: 120
    dangle_parens: false

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

You can also generate a full starter config with:

```bash
cmakefmt --dump-config
```

By default this emits YAML. Pass `toml` to print the TOML variant:

```bash
cmakefmt --dump-config toml
```

Legacy `cmake-format` config files can be converted with:

```bash
cmakefmt --convert-legacy-config path/to/.cmake-format.py > .cmakefmt.toml
```

Optional features that are off by default, such as tab indentation or extra
spacing before control-flow parentheses, are emitted as commented-out settings
in that template. Uncomment them when you want to opt in. `--dump-config`
emits YAML by default, while `--dump-config toml` prints the TOML variant.

Currently supported config sections:

- `[format]`
  - `line_width`
  - `tab_size`
  - `use_tabs`
  - `max_empty_lines`
  - `max_hanging_wrap_lines`
  - `max_hanging_wrap_positional_args`
  - `max_hanging_wrap_groups`
  - `dangle_parens`
  - `dangle_align`
  - `min_prefix_length`
  - `max_prefix_length`
  - `space_before_control_paren`
  - `space_before_definition_paren`
- `[style]`
  - `command_case`
  - `keyword_case`
- `[markup]`
  - `enable_markup`
  - `reflow_comments`
  - `first_comment_is_literal`
  - `literal_comment_pattern`
  - `bullet_char`
  - `enum_char`
  - `fence_pattern`
  - `ruler_pattern`
  - `hashruler_min_length`
  - `canonicalize_hashrulers`
- `[per_command.<name>]`
  - `command_case`
  - `keyword_case`
  - `line_width`
  - `tab_size`
  - `dangle_parens`
  - `dangle_align`
  - `max_hanging_wrap_positional_args`
  - `max_hanging_wrap_groups`
- `[commands.<name>]`
  - `pargs`
  - `flags`
  - `kwargs`

`per_command.<name>` changes formatting knobs for a known command name.
`commands.<name>` teaches `cmakefmt` the syntax of a custom command or
overrides the built-in shape of an existing one.

For user config, prefer YAML once custom command specs grow beyond a couple of
flags/kwargs. TOML remains supported and is what `--dump-config` currently
prints, but YAML is much easier to read and edit for larger nested command
specs.

The current `cmakefmt` config schema only accepts the clearer names above. If
you have an older local config draft using names like
`use_tabchars`, `max_pargs_hwrap`, or `separate_ctrl_name_with_space`, update
it to the new spellings before relying on it.

## Library usage

```rust
use cmakefmt::{format_source, Config};

fn main() -> Result<(), cmakefmt::Error> {
    let source = r#"target_link_libraries(foo PUBLIC bar)"#;
    let formatted = format_source(source, &Config::default())?;
    println!("{formatted}");
    Ok(())
}
```

## Development

Run the quality gates:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Run benchmarks:

```bash
cargo bench
```

Save a benchmark baseline:

```bash
cargo bench --bench formatter -- --save-baseline local
```

Compare against a saved baseline:

```bash
cargo bench --bench formatter -- --baseline local
```

Benchmark methodology and profiling notes live in
[docs/PERFORMANCE.md](docs/PERFORMANCE.md).

If you use pre-commit:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Validate the docs structure:

```bash
bash scripts/check-docs.sh
```

## Current limitations

- The real-world validation corpus is still limited to the checked-in sample
  set, even though the current outputs are in a good place for that corpus.
- Full built-in and module command coverage in `src/spec/builtins.toml` is
  still being audited and expanded.
- Very-large-codebase parallel surveying, release packaging, and
  package-manager distribution are not finished.
- Comment reflow is opt-in via `markup.reflow_comments`, and only line comments
  are wrapped today. More advanced markup-aware comment formatting is still
  less mature than the core formatting path.
