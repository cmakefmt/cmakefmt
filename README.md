<h1><code>cmakefmt</code></h1>

[![CI](https://github.com/puneetmatharu/cmakefmt/actions/workflows/ci.yml/badge.svg)](https://github.com/puneetmatharu/cmakefmt/actions/workflows/ci.yml)
[![Docs](https://github.com/puneetmatharu/cmakefmt/actions/workflows/docs.yml/badge.svg)](https://github.com/puneetmatharu/cmakefmt/actions/workflows/docs.yml)
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

- **20× faster — not a typo.** Geometric-mean speedup of `20.77x` over `cmake-format` on real-world corpora.
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
| `abseil/CMakeLists.txt`         |   204 |         4.467 |           114.576 |  25.65× |
| `catch2/CMakeLists.txt`         |   231 |         4.558 |           101.606 |  22.29× |
| `cli11/CMakeLists.txt`          |   283 |         4.458 |           118.954 |  26.68× |
| `nlohmann_json/CMakeLists.txt`  |   237 |         4.717 |           131.813 |  27.95× |
| `qtbase_network/CMakeLists.txt` |   420 |         5.557 |           279.420 |  50.28× |

Geometric-mean speedup across the full corpus: **`20.77×`**.
`--parallel 8` improves whole-corpus throughput by a further **`3.43×`**.

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
