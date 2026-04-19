<p align="center">
  <a href="https://cmakefmt.dev">
    <img src="https://raw.githubusercontent.com/cmakefmt/cmakefmt/main/assets/banner.png" alt="cmakefmt banner" width="100%"/>
  </a>
</p>

<h1><code>cmakefmt</code></h1>

<p align="center">
  <sup>CI</sup><br>
  <a href="https://github.com/cmakefmt/cmakefmt/actions/workflows/ci.yml"><img src="https://github.com/cmakefmt/cmakefmt/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>&nbsp;<a href="https://github.com/cmakefmt/cmakefmt/actions/workflows/pages.yml"><img src="https://github.com/cmakefmt/cmakefmt/actions/workflows/pages.yml/badge.svg?branch=main" alt="Pages" /></a>&nbsp;<a href="https://github.com/cmakefmt/cmakefmt/actions/workflows/coverage.yml"><img src="https://github.com/cmakefmt/cmakefmt/actions/workflows/coverage.yml/badge.svg?branch=main" alt="Coverage" /></a>&nbsp;<a href="https://codspeed.io/cmakefmt/cmakefmt?utm_source=badge"><img src="https://img.shields.io/endpoint?url=https://codspeed.io/badge.json" alt="CodSpeed" /></a>
</p>

<p align="center">
  <sup>Package</sup><br>
  <a href="https://crates.io/crates/cmakefmt-rust"><img src="https://img.shields.io/crates/v/cmakefmt-rust.svg" alt="Crates.io" /></a>&nbsp;<a href="https://deps.rs/repo/github/cmakefmt/cmakefmt"><img src="https://deps.rs/repo/github/cmakefmt/cmakefmt/status.svg?branch=main" alt="dependency status" /></a>
</p>

<p align="center">
  <sup>Security &amp; Quality</sup><br>
  <a href="https://api.reuse.software/info/github.com/cmakefmt/cmakefmt"><img src="https://api.reuse.software/badge/github.com/cmakefmt/cmakefmt" alt="REUSE status" /></a>&nbsp;<a href="https://securityscorecards.dev/viewer/?uri=github.com/cmakefmt/cmakefmt"><img src="https://api.securityscorecards.dev/projects/github.com/cmakefmt/cmakefmt/badge" alt="OpenSSF Scorecard" /></a>&nbsp;<a href="https://www.bestpractices.dev/projects/12392"><img src="https://www.bestpractices.dev/projects/12392/badge" alt="OpenSSF Best Practices" /></a>
</p>

**A lightning-fast, workflow-first CMake formatter — built in Rust, built to last.**

![cmakefmt demo](https://cmakefmt.dev/cmakefmt.gif)

The above demo was generated with [VHS](https://github.com/charmbracelet/vhs).

`cmakefmt` replaces the aging Python [`cmake-format`](https://github.com/cheshirekow/cmake_format) tool with a
single native binary. Same spirit. No Python.

* [crates.io](crates.io): `cmakefmt-rust`
* CLI name: `cmakefmt`

> [!NOTE]
>
> This project is independent from other Rust implementations, including: [`azais-corentin/cmakefmt`](https://github.com/azais-corentin/cmakefmt) and [`yamadapc/cmakefmt`](https://github.com/yamadapc/cmakefmt).

<h2>Contents</h2>

* [Why `cmakefmt`?](#why-cmakefmt)
* [Performance](#performance)
* [Installation](#installation)
* [Quick Start](#quick-start)
* [Common Workflows](#common-workflows)
* [Configuration](#configuration)
* [Formatter Disable Regions](#formatter-disable-regions)
* [Library Usage](#library-usage)
* [Documentation](#documentation)
* [Project Layout](#project-layout)
* [Development](#development)
* [Status](#status)
* [License](#license)

## Why `cmakefmt`?

* **48× faster — not a typo.** Geometric-mean speedup of `48×` over `cmake-format` on real-world per-file corpora,
  rising to **485×** on whole-repository runs. Pre-commit hooks that once made you wince now finish before you blink.
* **Zero dependencies. One binary.** No Python environment, no virtualenv bootstrap, no dependency drift.
  Drop it in CI and forget about it.
* **Built for actual workflows.** `--check`, `--diff`, `--staged`, `--changed`, `--files-from`,
  `--show-config`, `--explain-config`, semantic verification, JSON reporting — all first-class,
  not scripted workarounds.
* **Knows your commands.** Teach `cmakefmt` the argument structure of your project's custom CMake functions and macros.
  No more generic token-wrapping for code *you* wrote.
* **Errors that actually help.** Parse and config failures come with file/line context, source snippets,
  and reproduction hints — not opaque parser noise.
* **Designed for real repositories.** Comment preservation, disable-region passthrough, config discovery,
  ignore files, Git-aware file selection, and opt-in parallelism are core features, not afterthoughts.

## Performance

| Fixture                          |  Lines | `cmakefmt` ms | `cmake-format` ms |   Speedup |
|----------------------------------|-------:|--------------:|------------------:|----------:|
| `opencv_flann/CMakeLists.txt`    |      2 |          7.46 |             51.07 |      6.8× |
| `googletest/CMakeLists.txt`      |     36 |          4.82 |             63.10 |     13.1× |
| `llvm_tablegen/CMakeLists.txt`   |     83 |          8.81 |             76.55 |      8.7× |
| `abseil/CMakeLists.txt`          |    280 |         10.24 |            168.91 |     16.5× |
| `spdlog/CMakeLists.txt`          |    413 |         10.95 |            217.78 |     19.9× |
| `mariadb_server/CMakeLists.txt`  |    656 |         11.82 |            485.42 |     41.1× |
| `xnnpack/CMakeLists.txt`         |  1,354 |         17.57 |          1,429.83 |     81.4× |
| `opencv/CMakeLists.txt` (root)   |  2,039 |         26.34 |         38,910.13 |  1,477.2× |
| `blender/CMakeLists.txt` (root)  |  2,650 |         49.19 |          2,638.59 |     53.6× |
| `llvm/libc/.../CMakeLists.txt`   |  6,688 |         20.34 |          2,219.31 |    109.1× |
| `grpc/CMakeLists.txt` (root)     | 54,834 |        179.29 |         76,098.27 |    424.4× |

Geometric-mean per-file speedup: **`48×`**.
Whole-repository parallel runs (across 14 large open-source repos including
LLVM, blender, opencv, gRPC, and oomph-lib) average **`49×` faster than
`cmake-format`** geo-mean, with parallel mode delivering up to **`5.1×`** over
`cmakefmt` serial on large repositories.

Full methodology and profiler notes: [cmakefmt.dev/performance/](https://cmakefmt.dev/performance/).

Update the pinned local corpus and generate local before/after review artifacts with:

```bash
python3 scripts/fetch-real-world-corpus.py
scripts/review-real-world-corpus.sh
```

## Installation

**Homebrew (macOS):**

```bash
brew install cmakefmt/cmakefmt/cmakefmt
```

**Cargo (any platform):**

```bash
cargo install cmakefmt-rust
```

**pip (any platform with Python):**

```bash
pip install cmakefmt
```

**conda-forge:**

```bash
conda install -c conda-forge cmakefmt
```

**Pre-built binaries (Linux, macOS, and Windows):**

Download the `.zip` / `.tar.gz` for your platform from
[GitHub Releases](https://github.com/cmakefmt/cmakefmt/releases/latest),
extract, and place the binary on your `PATH`.

**Build from source:**

```bash
git clone https://github.com/cmakefmt/cmakefmt
cd cmakefmt
cargo install --path .
```

Verify:

```bash
cmakefmt --version
```

Release channels and support levels are documented at [cmakefmt.dev/release/](https://cmakefmt.dev/release/).
Shell completion installation instructions are available at [cmakefmt.dev/install/](https://cmakefmt.dev/install/).

## Quick Start

```bash
cmakefmt config init               # generate a starter .cmakefmt.yaml
cmakefmt --check .                 # just show which files would change
cmakefmt .                         # preview formatted output; changed lines shown in blue
cmakefmt --diff .                  # view a unified diff of what would change (like `git diff`)
cmakefmt --in-place .              # apply formatting across the whole project
cmakefmt --staged --check          # use in pre-commit hooks
cmakefmt --path-regex 'src/.*' .   # format CMake files only under src/
```

## Common Workflows

| Task                                        | Command                                          |
|---------------------------------------------|--------------------------------------------------|
| Format file to stdout                       | `cmakefmt CMakeLists.txt`                        |
| Rewrite files in place                      | `cmakefmt --in-place .`                          |
| CI check                                    | `cmakefmt --check .`                             |
| Preview which files would change            | `cmakefmt --list-changed-files .`                |
| See the exact patch                         | `cmakefmt --diff CMakeLists.txt`                 |
| Verify semantics while formatting to stdout | `cmakefmt --verify CMakeLists.txt`               |
| Pre-commit guard (staged files only)        | `cmakefmt --staged --check`                      |
| PR-scoped check                             | `cmakefmt --changed --since origin/main --check` |
| Machine-readable CI output                  | `cmakefmt --check --report-format json .`        |
| GitHub Actions annotations                  | `cmakefmt --check --report-format github .`      |
| Checkstyle / JUnit / SARIF output           | `cmakefmt --check --report-format checkstyle .`  |
| Pin the required binary version in CI       | `cmakefmt --required-version 1.0.0 --check .`    |
| Speed up repeated large-repo checks         | `cmakefmt --cache --check .`                     |
| Roll out formatting file-by-file            | `cmakefmt --require-pragma --check .`            |
| Find project-specific commands              | `cmakefmt --list-unknown-commands .`             |
| Watch and auto-format on save               | `cmakefmt --watch .`                             |
| Explain formatting decisions                | `cmakefmt --explain CMakeLists.txt`              |
| Read from stdin                             | `cat CMakeLists.txt \| cmakefmt -`               |

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
  enable_markup: true
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

Full config reference: [cmakefmt.dev/config/](https://cmakefmt.dev/config/).

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

Full API docs: [cmakefmt.dev/api/](https://cmakefmt.dev/api/).

## Documentation

Start here: [https://cmakefmt.dev](https://cmakefmt.dev).

| Doc                                                                  | Description                                                              |
|----------------------------------------------------------------------|--------------------------------------------------------------------------|
| [Install](https://cmakefmt.dev/install/)                             | Install options, first-project setup, CI wiring                          |
| [Coverage](https://cmakefmt.dev/coverage/)                           | How coverage is measured, published, and interpreted                     |
| [Release Channels](https://cmakefmt.dev/release/)                    | Release contract, support levels, and release artifacts                  |
| [CLI Reference](https://cmakefmt.dev/cli/)                           | Every flag, exit code, and discovery rule                                |
| [Config Reference](https://cmakefmt.dev/config/)                     | Full config schema with examples                                         |
| [Formatter Behavior](https://cmakefmt.dev/behavior/)                 | How the formatter makes layout decisions                                 |
| [Migration from `cmake-format`](https://cmakefmt.dev/migration/)     | Incremental rollout guide and CLI mapping                                |
| [Library API](https://cmakefmt.dev/api/)                             | Embedding `cmakefmt` in your own Rust tools                              |
| [Troubleshooting](https://cmakefmt.dev/troubleshooting/)             | Common issues and debug workflow                                         |
| [Performance](https://cmakefmt.dev/performance/)                     | Benchmark methodology and profiler notes                                 |
| [Contributing](CONTRIBUTING.md)                                      | How to contribute, run tests, and open PRs                               |
| [Changelog](CHANGELOG.md)                                            | What's changed in each release                                           |

Preview the full docs locally:

```bash
cd docs && npm install && npm run dev
```

## Project Layout

```text
cmakefmt/
├── docs/        # Astro + Starlight source published to cmakefmt.dev
├── src/         # CLI, library API, parser, config, spec, formatter
├── tests/       # integration tests, snapshots, and fixtures
├── benches/     # Criterion benchmarks
├── scripts/     # repo maintenance and docs helpers
└── .github/     # CI and Pages workflows
```

Key modules under `src/`:

* `main.rs`: CLI entry point and workflow orchestration
* `lib.rs`: public library API
* `config/`: config loading, merging, and legacy conversion
* `parser/`: `pest` grammar, AST, and parse pipeline
* `spec/`: built-in and user-defined command registry
* `formatter/`: AST-to-doc formatting logic and comment handling
* `files.rs`: file discovery and ignore handling

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

The repository is stable and actively maintained. `cmakefmt` is still
pre-`1.0`, so release packaging, package-manager distribution, and some output
or API details may continue to evolve. The built-in command registry is audited
through CMake 4.3.1.

Hit something unexpected? See [Troubleshooting](https://cmakefmt.dev/troubleshooting/) or run:

```bash
cmakefmt --debug --check path/to/CMakeLists.txt
```

## License

`cmakefmt` is dual-licensed under [MIT](LICENSES/MIT.txt) or [Apache-2.0](LICENSES/Apache-2.0.txt) at your option.
