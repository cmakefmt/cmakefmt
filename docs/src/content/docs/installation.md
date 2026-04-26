---
title: Installation
description: Install cmakefmt via Homebrew, Cargo, or prebuilt binaries on macOS, Linux, and Windows.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Get `cmakefmt` running, wire it into your project, and never worry about CMake
formatting again.

## Current Installation Options

### Homebrew

Recommended for macOS users — no Rust toolchain needed:

```bash
brew install cmakefmt/cmakefmt/cmakefmt
```

### Cargo

Reference install path for developers already using Rust, works on any platform:

```bash
cargo install cmakefmt-rust
```

Verify the binary is on your path:

```bash
cmakefmt --version
cmakefmt --help
```

You can also install from a local checkout — for development, benchmarking, or
reviewing changes:

```bash
cargo install --path .
```

### pip

```bash
pip install cmakefmt
```

This installs the native `cmakefmt` binary into your environment's `bin/`
directory. No Python runtime overhead — the binary is the same Rust-compiled
formatter available via Homebrew and Cargo.

Pre-built wheels are available for Linux (x86_64, aarch64), macOS (x86_64,
aarch64), and Windows (x64). On unsupported platforms, pip falls back to
building from the source distribution, which requires a Rust toolchain.

### conda-forge

```bash
conda install -c conda-forge cmakefmt
```

Built from source by conda-forge. Future version bumps are handled
automatically by the conda-forge autotick bot.

### winget

Recommended for Windows users — installs a native binary, no Rust
toolchain or Python environment needed:

```powershell
winget install cmakefmt.cmakefmt
```

The package is published in the [`microsoft/winget-pkgs`](https://github.com/microsoft/winget-pkgs/tree/master/manifests/c/cmakefmt/cmakefmt)
community repository. New manifests are submitted automatically when each
GitHub Release is published, so the available version closely tracks the
latest tagged release once the upstream PR is merged.

### Pre-built Binaries

Native binaries for Linux, macOS, and Windows are published to
[GitHub Releases](https://github.com/cmakefmt/cmakefmt/releases/latest).
Download the `.zip` / `.tar.gz` for your platform, extract, and place the
binary on your `PATH`.

### Build From Source

```bash
git clone https://github.com/cmakefmt/cmakefmt
cd cmakefmt
cargo build --release
./target/release/cmakefmt --help
```

This is the right path if you are actively developing `cmakefmt`, reviewing
changes, or benchmarking local modifications.

## Support Levels

The release plan separates channels into explicit support levels so users know
what to trust:

| Channel | Support level | Notes |
| --- | --- | --- |
| Homebrew (`cmakefmt/cmakefmt`) | Officially maintained | Recommended for macOS users. Ships completions and man page. |
| `cargo install cmakefmt-rust` | Officially maintained | Reference install path for developers already using Rust. |
| `pip install cmakefmt` | Officially maintained | Native binary via pre-built wheel. |
| `conda install -c conda-forge cmakefmt` | Community maintained | Built from source; autotick bot tracks releases. |
| `winget install cmakefmt.cmakefmt` | Officially maintained | Recommended on Windows. Manifest lives in `microsoft/winget-pkgs`; new versions are submitted automatically on each release. |
| GitHub Releases binaries | Officially maintained | Native binaries for Linux, macOS, and Windows. |
| Docs site / CLI reference | Officially maintained | Stays in lock-step with each tagged release. |
| Scoop | Planned | Not published yet. |
| Additional package managers (AUR, Nix, containers, etc.) | Automated or best-effort | Useful channels, but not the first rollout priority. |

## Shell Completions

Release archives include shell completion scripts for the supported shells:

- `cmakefmt.bash` for Bash
- `_cmakefmt` for Zsh
- `cmakefmt.fish` for Fish

The Zsh file intentionally uses the conventional completion-function name
`_cmakefmt` rather than a `.zsh` suffix.

You can also generate the completion files yourself from any installed binary:

```bash
cmakefmt completions bash > cmakefmt.bash
cmakefmt completions zsh > _cmakefmt
cmakefmt completions fish > cmakefmt.fish
```

### Bash

Source the file from your `.bashrc` or `.bash_profile`:

```bash
cmakefmt completions bash > ~/.local/share/bash-completion/completions/cmakefmt
```

Or for a system-wide install (requires write access):

```bash
cmakefmt completions bash | sudo tee /etc/bash_completion.d/cmakefmt > /dev/null
```

### Zsh

Place the file somewhere on your `fpath` and reload completions:

```bash
cmakefmt completions zsh > ~/.zfunc/_cmakefmt
```

Then add the following to your `.zshrc` if `~/.zfunc` is not already on `fpath`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

Fish looks for completions in a fixed directory — just drop the file there:

```bash
cmakefmt completions fish > ~/.config/fish/completions/cmakefmt.fish
```

Fish picks up the new file automatically without a shell restart.

## First Project Setup

Dump a starter config into your repo root:

```bash
cmakefmt config dump > .cmakefmt.yaml
```

Why YAML by default? For larger configs, YAML requires less punctuation and is
more readable with nested custom-command specs. TOML is still available via
`config dump --format toml` if you prefer it.

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

- start from `config dump`
- keep the file as `.cmakefmt.yaml`
- define command syntax under `commands:`
- use `per_command_overrides:` only for layout and style tweaks

If you are debugging config discovery:

```bash
cmakefmt config path src/CMakeLists.txt
cmakefmt config show src/CMakeLists.txt
cmakefmt config explain
```

## Upgrade And Uninstall

### Upgrade a Homebrew install

```bash
brew update
brew upgrade cmakefmt
```

### Remove a Homebrew install

```bash
brew uninstall cmakefmt
brew untap cmakefmt/cmakefmt
```

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
cmakefmt config path path/to/CMakeLists.txt
cmakefmt config explain
```

### A hook or script only sees stdin and ignores my project config

Pass `--stdin-path` with the buffer's real project-relative path.

### I want TOML instead of YAML

```bash
cmakefmt config dump --format toml > .cmakefmt.toml
```

YAML is the recommended default because it is more readable for larger configurations with nested custom command specs.
