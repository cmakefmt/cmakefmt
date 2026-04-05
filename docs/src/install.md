<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Install

Get `cmakefmt` running, wire it into your project, and never think about CMake
formatting again.

## Current Installation Options

The reference install path is `cargo install cmakefmt-rust`. It is
officially maintained and works today:

```bash
cargo install cmakefmt-rust
```

If you prefer to build from a local checkout — for development, benchmarking,
or reviewing changes — see [Build From This Repository](#build-from-this-repository) below.

First-party Homebrew and pre-built binary releases are being rolled out. Until
they land, Cargo is the fastest path to a working binary.

## Support Levels

The release plan separates channels into explicit support levels so users know
what to trust:

| Channel | Planned support level | Notes |
|---------|------------------------|-------|
| `cargo install cmakefmt-rust` | Officially maintained | The reference install path for developers already using Rust. |
| GitHub Releases binaries | Officially maintained | Native binaries for Linux, macOS, and Windows. |
| Docs site / CLI reference | Officially maintained | Should stay in lock-step with each tagged release. |
| Homebrew / `winget` / Scoop | Officially maintained | Planned first-party package-manager channels. |
| Additional package managers (`npm`, AUR, Nix, containers, etc.) | Automated or best-effort | Useful channels, but not the first rollout priority. |

Until tagged distribution channels land, repository-based installs remain the
fully supported path.

## Build From This Repository

```bash
git clone https://github.com/cmakefmt/cmakefmt
cd cmakefmt
cargo build --release
./target/release/cmakefmt --help
```

This is the right path if you are actively developing `cmakefmt`, reviewing
changes, or benchmarking local modifications.

## Install With Cargo

### From crates.io (recommended)

```bash
cargo install cmakefmt-rust
```

Verify the binary is on your path:

```bash
cmakefmt --version
cmakefmt --help
```

### From a local checkout

```bash
cargo install --path .
```

This is the right path when you have cloned the repository and want to run the
exact version you have checked out — for development, reviewing changes, or
benchmarking local modifications. The verify step above applies here too.

You can also inspect release-oriented helper outputs directly from the built
binary:

```bash
cmakefmt --generate-completion bash > cmakefmt.bash
cmakefmt --generate-man-page > cmakefmt.1
```

Those outputs are intended for packaging and release artifacts, but they are
also useful for local shell setup.

## Shell Completions

Release archives include shell completion scripts for the supported shells:

- `cmakefmt.bash` for Bash
- `_cmakefmt` for Zsh
- `cmakefmt.fish` for Fish

The Zsh file intentionally uses the conventional completion-function name
`_cmakefmt` rather than a `.zsh` suffix.

You can also generate the completion files yourself from any installed binary:

```bash
cmakefmt --generate-completion bash > cmakefmt.bash
cmakefmt --generate-completion zsh > _cmakefmt
cmakefmt --generate-completion fish > cmakefmt.fish
```

### Bash

Source the file from your `.bashrc` or `.bash_profile`:

```bash
cmakefmt --generate-completion bash > ~/.local/share/bash-completion/completions/cmakefmt
```

Or for a system-wide install (requires write access):

```bash
cmakefmt --generate-completion bash | sudo tee /etc/bash_completion.d/cmakefmt > /dev/null
```

### Zsh

Place the file somewhere on your `fpath` and reload completions:

```bash
cmakefmt --generate-completion zsh > ~/.zfunc/_cmakefmt
```

Then add the following to your `.zshrc` if `~/.zfunc` is not already on `fpath`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

Fish looks for completions in a fixed directory — just drop the file there:

```bash
cmakefmt --generate-completion fish > ~/.config/fish/completions/cmakefmt.fish
```

Fish picks up the new file automatically without a shell restart.

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

The commands you will reach for every day:

```bash
cmakefmt --check .
cmakefmt --in-place .
cmakefmt --verify CMakeLists.txt
cmakefmt --cache --check .
cmakefmt --require-pragma --check .
cmakefmt --staged --check
cmakefmt --changed --since origin/main --check
```

What each one does:

- `--check .`: CI-safe validation for a repository or directory
- `--in-place .`: rewrite all discovered CMake files, with semantic verification by default
- `--verify CMakeLists.txt`: do a safe stdout-format run when you want the extra parse-tree check
- `--cache --check .`: speed up repeated whole-repo checks when your config is stable
- `--require-pragma --check .`: roll formatting out gradually, only touching opted-in files
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
cmakefmt --explain-config
```

## Local Docs Preview

Preview the published docs locally with `mdbook`:

```bash
mdbook serve docs
```

Then open the local URL that `mdbook` prints.

## Upgrade And Uninstall

### Upgrade a local source install

```bash
git pull --ff-only
cargo install --path . --force
```

### Remove a Cargo-installed binary

```bash
cargo uninstall cmakefmt-rust
```

### Pin a specific release in CI later

Once release tags exist, prefer explicit version pins:

```bash
cargo install cmakefmt-rust --version <tagged-version>
```

The release docs and release notes will also publish SHA-256 sums for release
artifacts so non-Cargo installs can verify downloads.

## Troubleshooting Install Issues

### `cmakefmt` is not found after `cargo install`

Make sure Cargo's install bin directory is on your `PATH`.

### The formatter is using the wrong config

```bash
cmakefmt --show-config-path path/to/CMakeLists.txt
cmakefmt --explain-config
```

### A hook or script only sees stdin and ignores my project config

Pass `--stdin-path` with the buffer's real project-relative path.

### I want TOML instead of YAML

```bash
cmakefmt --dump-config toml > .cmakefmt.toml
```

YAML is simply the recommended default for larger configs.
