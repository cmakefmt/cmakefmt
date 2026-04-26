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

**A fast, correct CMake formatter — `cmake-format`, reimagined in Rust.**

![cmakefmt demo](https://cmakefmt.dev/cmakefmt.gif)

<h2>Contents</h2>

- [Why `cmakefmt`?](#why-cmakefmt)
- [Install](#install)
- [Quick start](#quick-start)
- [GitHub Action](#github-action)
- [Documentation](#documentation)
- [Status](#status)
- [License](#license)

## Why `cmakefmt`?

- **Drop-in replacement for `cmake-format`.** A single native binary with the
  same workflows and no Python environment to manage.
- **100× faster.** Geometric-mean speedup over `cmake-format` across a corpus
  of 14 large open-source repositories (see [Performance](https://cmakefmt.dev/performance/)).
- **Workflow-first design.** `--check`, `--diff`, `--staged`, `--changed`,
  semantic verification, JSON/SARIF/JUnit reports, an LSP server, and a
  [GitHub Action](#github-action) are all first-class features rather than
  scripted afterthoughts.

## Install

```bash
brew install cmakefmt/cmakefmt/cmakefmt   # macOS / Linux (Homebrew)
pip install cmakefmt                      # any platform with Python
cargo install cmakefmt-rust               # any platform
winget install cmakefmt.cmakefmt          # Windows
```

Conda, Docker, pre-built binaries, and full setup notes are documented at
[Installation](https://cmakefmt.dev/installation/).

## Quick start

```bash
cmakefmt --in-place .   # format every CMake file in the project
```

The full CLI reference, configuration schema, editor integrations, and
migration guide from `cmake-format` all live at
[**cmakefmt.dev**](https://cmakefmt.dev).

## GitHub Action

The official [`cmakefmt-action`](https://github.com/cmakefmt/cmakefmt-action)
wraps the binary for use in CI workflows:

```yaml
- uses: actions/checkout@v6
- uses: cmakefmt/cmakefmt-action@v2
```

That snippet is the strict whole-repo check: it fails the job and emits inline
PR annotations for any file that would change under formatting. The action
also supports other modes and file-selection scopes:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    mode: diff       # check, diff, fix, or setup
    scope: changed   # all, changed, or staged
```

For the complete list of inputs and recommended rollout patterns, see the
[`cmakefmt-action` README](https://github.com/cmakefmt/cmakefmt-action#readme).

## Documentation

The full documentation lives at [**cmakefmt.dev**](https://cmakefmt.dev).

| Page                                                             | Description                           |
|------------------------------------------------------------------|---------------------------------------|
| [Getting Started](https://cmakefmt.dev/getting-started/)         | First format in under a minute        |
| [Installation](https://cmakefmt.dev/installation/)               | Every install channel and setup notes |
| [CLI Reference](https://cmakefmt.dev/cli/)                       | Every flag, subcommand, exit code     |
| [Config Reference](https://cmakefmt.dev/config/)                 | Full config schema with examples      |
| [Migration from `cmake-format`](https://cmakefmt.dev/migration/) | Incremental rollout guide             |
| [Editor Integration](https://cmakefmt.dev/editors/)              | VS Code, Neovim, Helix, Zed, Emacs    |
| [Comparison](https://cmakefmt.dev/comparison/)                   | vs `cmake-format` and `gersemi`       |
| [Performance](https://cmakefmt.dev/performance/)                 | Benchmark methodology and numbers     |
| [Library API](https://cmakefmt.dev/api/)                         | Embed `cmakefmt` in Rust code         |
| [Troubleshooting](https://cmakefmt.dev/troubleshooting/)         | Common issues and debug workflow      |
| [Playground](https://cmakefmt.dev/playground/)                   | Try `cmakefmt` in your browser        |
| [Contributing](CONTRIBUTING.md) / [Changelog](CHANGELOG.md)      | How to help; what changed             |

## Status

`cmakefmt` is stable and actively maintained. The built-in command spec is
audited against CMake 4.3.1. The release contract and per-channel support
levels are documented at [Release](https://cmakefmt.dev/release/).

## License

`cmakefmt` is dual-licensed under [MIT](LICENSES/MIT.txt) or
[Apache-2.0](LICENSES/Apache-2.0.txt), at your option.

---

<sub>This project is independent from other Rust crates of the same name —
it is not affiliated with
[`azais-corentin/cmakefmt`](https://github.com/azais-corentin/cmakefmt) or
[`yamadapc/cmakefmt`](https://github.com/yamadapc/cmakefmt).</sub>
