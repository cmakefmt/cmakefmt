---
title: Getting Started
description: Install cmakefmt and format your first CMake file in under a minute.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` is a fast, workflow-first formatter for CMake files. This page gets you from
installation to a fully formatted repository in under a minute.

## Install

**Homebrew** — recommended for macOS, no Rust toolchain needed:

```bash
brew install cmakefmt/cmakefmt/cmakefmt
```

**Pre-built binaries** — for Linux, macOS, and Windows, download the `.zip` / `.tar.gz`
from [GitHub Releases](https://github.com/cmakefmt/cmakefmt/releases/latest),
extract, and place the binary on your `PATH`.

**Cargo** — for developers already using Rust, works on any platform:

```bash
cargo install cmakefmt-rust
```

Verify the install:

```bash
cmakefmt --version
```

For shell completions, editor setup, and more install options see the full [Installation](/installation/) page.

## Generate a Config

Dump a starter config to your repository root:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

YAML is the recommended format. Open the file and adjust options like
`format.line_width` and `style.command_case` to match your project's style.
See the [Config Reference](/config/) for every available option.

## Your First Run

Check what would change across your whole repository without touching any files:

```bash
cmakefmt --check .
```

If the output looks right, apply formatting:

```bash
cmakefmt --in-place .
```

## Pre-commit Hook

Format only the files you're about to commit:

```bash
cmakefmt --staged --check
```

The repository ships a `pre-commit` configuration. Install it once:

```bash
pre-commit install
```

## What's Next

- [Installation](/installation/) — full install options, shell completions, editor setup
- [CLI Reference](/cli/) — every flag documented
- [Config Reference](/config/) — tune `cmakefmt` for your project
- [Formatter Behavior](/behavior/) — understand what gets changed and why
- [Migration From `cmake-format`](/migration/) — coming from the Python tool?
