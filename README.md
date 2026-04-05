<p align="center">
  <img src="assets/logo.png" alt="cmakefmt logo" width="100%"/>
</p>

<h1><code>cmakefmt</code></h1>

[![CI](https://github.com/puneetmatharu/cmakefmt/actions/workflows/ci.yml/badge.svg)](https://github.com/puneetmatharu/cmakefmt/actions/workflows/ci.yml)
[![Pages](https://github.com/puneetmatharu/cmakefmt/actions/workflows/pages.yml/badge.svg)](https://github.com/puneetmatharu/cmakefmt/actions/workflows/pages.yml)
[![Coverage](https://github.com/puneetmatharu/cmakefmt/actions/workflows/coverage.yml/badge.svg)](https://github.com/puneetmatharu/cmakefmt/actions/workflows/coverage.yml)

**A blazing-fast, workflow-first CMake formatter — built in Rust, built to last.**

`cmakefmt` replaces the aging Python [`cmake-format`](https://github.com/cheshirekow/cmake_format) tool with a
single native binary. Same spirit. No Python. No compromises.

<h2>Contents</h2>

- [Why `cmakefmt`?](#why-cmakefmt)
- [Performance](#performance)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Common Workflows](#common-workflows)
- [Configuration](#configuration)
- [Formatter Disable Regions](#formatter-disable-regions)
- [Library Usage](#library-usage)
- [Documentation](#documentation)
- [Development](#development)
- [Status](#status)
- [License](#license)

## Why `cmakefmt`?

- **20× faster — not a typo.** Geometric-mean speedup of `20.69x` over `cmake-format` on real-world corpora.
  Pre-commit hooks that once made you wince now finish before you blink.
- **Zero dependencies. One binary.** No Python environment, no virtualenv bootstrap, no dependency drift.
  Drop it in CI and forget about it.
- **Built for actual workflows.** `--check`, `--diff`, `--staged`, `--changed`, `--files-from`,
  `--show-config`, `--explain-config`, semantic verification, JSON reporting — all first-class,
  not scripted workarounds.
- **Knows your commands.** Teach `cmakefmt` the shape of your project's custom CMake functions and macros.
  No more generic token-wrapping for code *you* wrote.
- **Errors that actually help.** Parse and config failures come with file/line context, source snippets,
  and reproduction hints — not opaque parser noise.
- **Designed for real repositories.** Comment preservation, disable-region passthrough, config discovery,
  ignore files, Git-aware file selection, and opt-in parallelism are core features, not afterthoughts.

## Performance

| Fixture                         | Lines | `cmakefmt` ms | `cmake-format` ms | Speedup |
|---------------------------------|------:|--------------:|------------------:|--------:|
| `abseil/CMakeLists.txt`         |   280 |         5.804 |           168.570 |  29.04× |
| `catch2/CMakeLists.txt`         |   230 |         5.768 |           105.614 |  18.31× |
| `cli11/CMakeLists.txt`          |   283 |         5.570 |           120.994 |  21.72× |
| `cmake_cmbzip2/CMakeLists.txt`  |    25 |         5.042 |            61.751 |  12.25× |
| `googletest/CMakeLists.txt`     |    36 |         5.004 |            62.439 |  12.48× |
| `ggml/CMakeLists.txt`           |   498 |         7.773 |           210.200 |  27.04× |
| `llama_cpp/CMakeLists.txt`      |   286 |         6.257 |           126.584 |  20.23× |
| `llvm_tablegen/CMakeLists.txt`  |    83 |         5.172 |            75.429 |  14.58× |
| `mariadb_server/CMakeLists.txt` |   656 |         9.774 |           473.879 |  48.49× |
| `nlohmann_json/CMakeLists.txt`  |   237 |         5.705 |           138.936 |  24.35× |
| `opencv_flann/CMakeLists.txt`   |     2 |         4.719 |            51.497 |  10.91× |
| `protobuf/CMakeLists.txt`       |   351 |         6.226 |           111.802 |  17.96× |
| `spdlog/CMakeLists.txt`         |   413 |         9.204 |           213.649 |  23.21× |
| `qtbase_network/CMakeLists.txt` |   420 |         8.146 |           284.355 |  34.91× |

Geometric-mean speedup across the full corpus: **`20.69×`**.
On a 220-file batch, `--parallel 8` improves throughput by **`3.80×`** vs serial.

Full methodology and profiler notes: [docs/PERFORMANCE.md](docs/PERFORMANCE.md).

Refresh the pinned local corpus and generate local before/after review artefacts with:

```bash
python3 scripts/fetch-real-world-corpus.py
scripts/review-real-world-corpus.sh
```

## Installation

`cmakefmt` has not yet reached its public alpha release. Until then, build from this repository:

```bash
git clone <this-repo>
cd cmake-format-rust
cargo install --path .
```

Verify:

```bash
cmakefmt --version
```

Package-manager distribution (Homebrew, crates.io, pre-built binaries) is coming in the alpha-release phase.

Planned release channels and support levels are documented in [site/src/release.md](site/src/release.md).

## Quick Start

**1. Generate a starter config in your project root:**

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

**2. Dry-run — check your whole project without touching any files:**

```bash
cmakefmt --check .
```

**3. Apply formatting:**

```bash
cmakefmt --in-place .
```

**4. Format only the files you're about to commit:**

```bash
cmakefmt --staged --check
```

## Common Workflows

| Task | Command |
|------|---------|
| Format file to stdout | `cmakefmt CMakeLists.txt` |
| Rewrite files in place | `cmakefmt --in-place .` |
| CI check | `cmakefmt --check .` |
| Preview which files would change | `cmakefmt --list-changed-files .` |
| See the exact patch | `cmakefmt --diff CMakeLists.txt` |
| Verify semantics while formatting to stdout | `cmakefmt --verify CMakeLists.txt` |
| Pre-commit guard (staged files only) | `cmakefmt --staged --check` |
| PR-scoped check | `cmakefmt --changed --since origin/main --check` |
| Machine-readable CI output | `cmakefmt --check --report-format json .` |
| GitHub Actions annotations | `cmakefmt --check --report-format github .` |
| Checkstyle / JUnit / SARIF output | `cmakefmt --check --report-format checkstyle .` |
| Pin the required binary version in CI | `cmakefmt --required-version 0.1.0 --check .` |
| Speed up repeated large-repo checks | `cmakefmt --cache --check .` |
| Roll out formatting file-by-file | `cmakefmt --require-pragma --check .` |
| Read from stdin | `cat CMakeLists.txt \| cmakefmt -` |

## Configuration

`cmakefmt` searches upward from each file for `.cmakefmt.yaml`, `.cmakefmt.yml`, or `.cmakefmt.toml`.
YAML is recommended for larger configs.

Example `.cmakefmt.yaml`:

```yaml
format:
  line_width: 100
  tab_size: 4

style:
  command_case: lower
  keyword_case: upper

markup:
  reflow_comments: true
```

Debug which config a file is actually using:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --explain-config
```

Migrate from an existing `cmake-format` config:

```bash
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.yaml
```

Full config reference: [site/src/config.md](site/src/config.md).

## Formatter Disable Regions

Selectively opt out of formatting with barrier comments.

There are three barrier styles:

1. Legacy `cmake-format` directives:

    ```cmake
    # cmake-format: off
    set(MESSY_THING  a   b   c)   # kept verbatim
    # cmake-format: on
    ```

1. Native directive barriers, using either `cmakefmt` or the shorter `fmt` spelling:

    ```cmake
    # cmakefmt: off
    set(MESSY_THING  a   b   c)   # kept verbatim
    # cmakefmt: on

    # fmt: off
    set(MESSY_THING  a   b   c)   # kept verbatim
    # fmt: on
    ```

1. Fence barriers, which toggle formatting on and off each time `# ~~~` appears:

    ```cmake
    # ~~~
    set(MESSY_THING  a   b   c)   # kept verbatim
    # ~~~
    ```

Use directive barriers when you want an explicit start/end marker, and fence
barriers when you want a shorter toggle-style block.

## Library Usage

`cmakefmt` is also available as a Rust library:

```rust
use cmakefmt::{format_source, Config};

fn main() -> Result<(), cmakefmt::Error> {
    let src = r#"target_link_libraries(foo PUBLIC bar baz)"#;
    let out = format_source(src, &Config::default())?;
    println!("{out}");
    Ok(())
}
```

Full API docs: [site/src/api.md](site/src/api.md).

## Documentation

Start here: [Docs Landing Page](site/src/README.md).

| Doc | Description |
|-----|-------------|
| [Install](site/src/install.md) | Install options, first-project setup, CI wiring |
| [Coverage](site/src/coverage.md) | How coverage is measured, published, and interpreted |
| [Release Channels](site/src/release.md) | Alpha contract, support levels, release artifacts, and shell completions |
| [CLI Reference](site/src/cli.md) | Every flag, exit code, and discovery rule |
| [Config Reference](site/src/config.md) | Full config schema with examples |
| [Formatter Behavior](site/src/behavior.md) | How the formatter makes layout decisions |
| [Migration from `cmake-format`](site/src/migration.md) | Incremental rollout guide and CLI mapping |
| [Library API](site/src/api.md) | Embedding `cmakefmt` in your own Rust tools |
| [Troubleshooting](site/src/troubleshooting.md) | Common issues and debug workflow |
| [Performance](docs/PERFORMANCE.md) | Benchmark methodology and profiler notes |
| [Contributing](CONTRIBUTING.md) | How to contribute, run tests, and open PRs |
| [Changelog](CHANGELOG.md) | What's changed in each release |

Preview the full docs locally:

```bash
mdbook serve site
```

## Development

```bash
cargo fmt --check                          # formatting
cargo clippy --all-targets -- -D warnings  # lints
cargo test                                 # all tests
cargo llvm-cov --workspace --all-targets   # coverage
cargo bench                                # benchmarks
```

Install pre-commit hooks:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

## Status

`cmakefmt` is pre-`1.0` — honest about it, but already genuinely useful.
The formatter is actively developed; large-codebase parallel surveying, release packaging,
and package-manager distribution are still in progress.

Hit something unexpected? See [Troubleshooting](site/src/troubleshooting.md) or run:

```bash
cmakefmt --debug --check path/to/CMakeLists.txt
```

## License

`cmakefmt` is dual-licensed under [MIT](LICENSES/MIT.txt) or [Apache-2.0](LICENSES/Apache-2.0.txt) at your option.
