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
- configuration via `.cmake-format.toml`
- a built-in command registry audited through CMake 4.3.1

The formatter is still under active development. Real-world corpus coverage and
full built-in/module command coverage and final performance work are not
complete yet.

The command spec version and audit date are stored in
`src/spec/builtins.toml` under `[metadata]`.

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
cmakefmt --dry-run path/to/project
```

Restrict recursive discovery with a regex:

```bash
cmakefmt --list-files --file-regex 'modules|toolchain' .
```

Read from stdin:

```bash
cat CMakeLists.txt | cmakefmt -
```

Use an explicit config file:

```bash
cmakefmt --config path/to/.cmake-format.toml CMakeLists.txt
```

Override config values on the command line:

```bash
cmakefmt --line-width 100 --tab-size 4 --command-case lower --keyword-case upper CMakeLists.txt
```

Print the default config template:

```bash
cmakefmt --dump-config
```

Print debug diagnostics while checking a file:

```bash
cmakefmt --debug --check CMakeLists.txt
```

Current CLI flags:

```text
cmakefmt [OPTIONS] [FILES]...

  -i, --in-place
      --check
      --list-files
      --dry-run
  -f, --file-regex <REGEX>
      --dump-config
      --debug
      --parallel [<JOBS>]
      --config <PATH>
      --line-width <N>
      --tab-size <N>
      --command-case <lower|upper|unchanged>
      --keyword-case <lower|upper|unchanged>
      --dangle-parens <true|false>
  -h, --help
  -V, --version
```

Exit codes:

- `0`: success
- `1`: `--check` or `--list-files` found files that would change
- `2`: parse, config, or I/O error

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

## Configuration

The formatter looks for `.cmake-format.toml` in this order:

1. `--config <PATH>` if provided
2. the directory of the file being formatted
3. parent directories up to the git root or filesystem root
4. `~/.cmake-format.toml`
5. built-in defaults

Example config:

```toml
[format]
line_width = 100
tab_size = 4
use_tabchars = false
max_empty_lines = 1
dangle_parens = true
dangle_align = "prefix"
separate_ctrl_name_with_space = false
separate_fn_name_with_space = false

[style]
command_case = "lower"
keyword_case = "upper"

[markup]
enable_markup = true
reflow_comments = true
first_comment_is_literal = true

[per_command.message]
line_width = 120
dangle_parens = false
```

You can also generate a full starter config with:

```bash
cmakefmt --dump-config
```

Optional features that are off by default, such as tab indentation or extra
spacing before control-flow parentheses, are emitted as commented-out settings
in that template. Uncomment them when you want to opt in.

Currently supported config sections:

- `[format]`
  - `line_width`
  - `tab_size`
  - `use_tabchars`
  - `max_empty_lines`
  - `max_lines_hwrap`
  - `max_pargs_hwrap`
  - `max_subgroups_hwrap`
  - `dangle_parens`
  - `dangle_align`
  - `min_prefix_chars`
  - `max_prefix_chars`
  - `separate_ctrl_name_with_space`
  - `separate_fn_name_with_space`
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
  - `max_pargs_hwrap`
  - `max_subgroups_hwrap`

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
[docs/PERFORMANCE.md](/Users/PuneetMatharu/Dropbox/programming/rust/cmake-format-rust/docs/PERFORMANCE.md).

If you use pre-commit:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

## Current limitations

- The real-world validation corpus is still limited to the checked-in sample
  set, even though the current outputs are in a good place for that corpus.
- Full built-in and module command coverage in `src/spec/builtins.toml` is
  still being audited and expanded.
- Benchmarking, release packaging, and package-manager distribution are not
  finished.
- Comment reflow is opt-in via `markup.reflow_comments`, and only line comments
  are wrapped today. More advanced markup-aware comment formatting is still
  less mature than the core formatting path.
