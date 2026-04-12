---
title: Install via pip
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Install the `cmakefmt` CLI binary via pip:

```bash
pip install cmakefmt
```

This installs the native `cmakefmt` binary into your environment's `bin/`
directory. No Python runtime overhead — the binary is the same Rust-compiled
formatter available via Homebrew and Cargo.

## Verify

```bash
cmakefmt --version
cmakefmt --help
```

## Usage

```bash
cmakefmt --check .                        # dry-run: show what would change
cmakefmt --in-place .                     # apply formatting
cmakefmt --staged --check                 # pre-commit: format only staged files
```

See the [CLI Reference](/cli/) for the full list of flags and options.

## Pre-built Wheels

Wheels are available for:

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x64)

On unsupported platforms, `pip install cmakefmt` falls back to building from
the source distribution, which requires a Rust toolchain.

## Virtual Environments

The binary is installed into the active virtual environment:

```bash
python -m venv .venv
source .venv/bin/activate
pip install cmakefmt
cmakefmt --version
```

## Pre-commit Integration

If you use the [pre-commit](https://pre-commit.com/) framework, you can also
install `cmakefmt` as a pre-commit hook — see the [CI page](/ci/#pre-commit).
