# Install

This page covers the practical ways to get `cmakefmt` running today, how to
bootstrap a project config, and how to wire the formatter into local workflows.

## Current Installation Options

`cmakefmt` has not reached its public alpha release yet, so the supported
installation paths today are repository-based:

- build from source with `cargo build --release`
- install from this checkout with `cargo install --path .`

First-party package-manager distribution is planned for the alpha-release
phase. Until then, expect Cargo or a source checkout to be the main path.

## Build From This Repository

```bash
git clone <this-repo>
cd cmake-format-rust
cargo build --release
./target/release/cmakefmt --help
```

This is the best path if you are actively developing `cmakefmt`, reviewing
changes, or benchmarking local modifications.

## Install With Cargo

```bash
cargo install --path .
```

After that, verify the binary:

```bash
cmakefmt --version
cmakefmt --help
```

## First Project Setup

Generate a starter config:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

Why YAML by default?

- it is easier to read for larger custom-command specs
- it is the recommended user-facing format for `cmakefmt`
- `--dump-config toml` still exists if you prefer TOML

Then check your project without rewriting anything:

```bash
cmakefmt --check .
```

If the output looks good, rewrite files in place:

```bash
cmakefmt --in-place .
```

## Typical Local Workflow

The most common day-to-day commands are:

```bash
cmakefmt --check .
cmakefmt --in-place .
cmakefmt --staged --check
cmakefmt --changed --since origin/main --check
```

What each one is for:

- `--check .`: CI-safe validation for a repository or directory
- `--in-place .`: rewrite all discovered CMake files
- `--staged --check`: quick pre-commit guard for staged files only
- `--changed --since origin/main --check`: review branch-only changes in a PR workflow

## Pre-commit

The repository already ships a `pre-commit` configuration. Install both the
normal commit hooks and the pre-push hooks:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Useful manual spot checks:

```bash
pre-commit run --all-files
cmakefmt --staged --check
```

That shipped hook set covers both code-quality checks and REUSE/license
metadata checks, so it is worth installing early in a contributor workflow.

## CI-Friendly Shell Usage

For a plain shell-based CI job, this is the simplest baseline:

```bash
cmakefmt --check .
```

If you want quieter output in CI logs:

```bash
cmakefmt --check --quiet .
```

If you want machine-readable output:

```bash
cmakefmt --check --report-format json .
```

## Editor And Stdin Workflows

Many editor integrations format a buffer through stdin instead of giving the
tool a real path. In that case, use `--stdin-path` so config discovery and
diagnostics still behave as if the file lived on disk:

```bash
cat src/CMakeLists.txt | cmakefmt - --stdin-path src/CMakeLists.txt
```

That is also the right pattern for ad-hoc scripts and editor commands.

## Config Bootstrap Tips

If your project uses many custom CMake functions/macros:

- start from `--dump-config`
- keep the file as `.cmakefmt.yaml`
- define syntax under `commands:`
- use `per_command_overrides:` only for layout/style tweaks

If you are debugging config discovery:

```bash
cmakefmt --show-config-path src/CMakeLists.txt
cmakefmt --show-config src/CMakeLists.txt
cmakefmt --explain-config src/CMakeLists.txt
```

## Local Docs Preview

Use `mdbook` to preview the published docs locally:

```bash
mdbook serve site
```

Then open the local URL that `mdbook` prints.

## Troubleshooting Install Issues

### `cmakefmt` is not found after `cargo install`

Make sure Cargo's install bin directory is on your `PATH`.

### The formatter is using the wrong config

Use:

```bash
cmakefmt --show-config-path path/to/CMakeLists.txt
cmakefmt --explain-config path/to/CMakeLists.txt
```

### A hook or script only sees stdin and ignores my project config

Use `--stdin-path` with the buffer's real project-relative path.

### I want TOML instead of YAML

That is supported:

```bash
cmakefmt --dump-config toml > .cmakefmt.toml
```

YAML is simply the recommended default for larger configs.
