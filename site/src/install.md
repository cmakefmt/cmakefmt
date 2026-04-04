# Install

Get `cmakefmt` running, wire it into your project, and never think about CMake
formatting again.

## Current Installation Options

`cmakefmt` has not yet reached its public alpha release, so the supported paths
today are repository-based:

- build from source with `cargo build --release`
- install from this checkout with `cargo install --path .`

First-party package-manager distribution is coming in the alpha-release phase.
Until then, Cargo is the fastest path to a working binary.

## Build From This Repository

```bash
git clone <this-repo>
cd cmake-format-rust
cargo build --release
./target/release/cmakefmt --help
```

This is the right path if you are actively developing `cmakefmt`, reviewing
changes, or benchmarking local modifications.

## Install With Cargo

```bash
cargo install --path .
```

Verify the binary is on your path:

```bash
cmakefmt --version
cmakefmt --help
```

## First Project Setup

Dump a starter config into your repo root:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

Why YAML by default?

- it is easier to read for larger custom-command specs
- it is the recommended user-facing format for `cmakefmt`
- `--dump-config toml` still exists if you prefer TOML

Do a dry run — check your whole project without rewriting a single file:

```bash
cmakefmt --check .
```

When you are happy with what you see, apply the formatting:

```bash
cmakefmt --in-place .
```

## Typical Local Workflow

The four commands you will reach for every day:

```bash
cmakefmt --check .
cmakefmt --in-place .
cmakefmt --staged --check
cmakefmt --changed --since origin/main --check
```

What each one does:

- `--check .`: CI-safe validation for a repository or directory
- `--in-place .`: rewrite all discovered CMake files
- `--staged --check`: pre-commit guard — only touches staged files
- `--changed --since origin/main --check`: PR-scoped check for branch-only changes

## Pre-commit

The repository ships a `pre-commit` configuration out of the box. Install both
commit and pre-push hooks:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Useful spot checks:

```bash
pre-commit run --all-files
cmakefmt --staged --check
```

The shipped hook set covers code-quality checks and REUSE/license metadata
validation — worth installing early in any contributor workflow.

## CI-Friendly Shell Usage

The simplest CI baseline:

```bash
cmakefmt --check .
```

For quieter CI logs:

```bash
cmakefmt --check --quiet .
```

For machine-readable output that scripts or dashboards can consume:

```bash
cmakefmt --check --report-format json .
```

## Editor And Stdin Workflows

Many editor integrations pipe a buffer through stdin rather than passing a real
file path. Use `--stdin-path` to give config discovery and diagnostics the
on-disk context they need:

```bash
cat src/CMakeLists.txt | cmakefmt - --stdin-path src/CMakeLists.txt
```

This is also the right pattern for ad-hoc scripts and custom editor commands.

## Config Bootstrap Tips

If your project uses many custom CMake functions or macros:

- start from `--dump-config`
- keep the file as `.cmakefmt.yaml`
- define command syntax under `commands:`
- use `per_command_overrides:` only for layout and style tweaks

If you are debugging config discovery:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --explain-config src/CMakeLists.txt
```

## Local Docs Preview

Preview the published docs locally with `mdbook`:

```bash
mdbook serve site
```

Then open the local URL that `mdbook` prints.

## Troubleshooting Install Issues

### `cmakefmt` is not found after `cargo install`

Make sure Cargo's install bin directory is on your `PATH`.

### The formatter is using the wrong config

```bash
cmakefmt --show-config-path path/to/CMakeLists.txt
cmakefmt --explain-config path/to/CMakeLists.txt
```

### A hook or script only sees stdin and ignores my project config

Pass `--stdin-path` with the buffer's real project-relative path.

### I want TOML instead of YAML

```bash
cmakefmt --dump-config toml > .cmakefmt.toml
```

YAML is simply the recommended default for larger configs.
