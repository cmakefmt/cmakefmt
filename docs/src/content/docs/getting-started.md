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

Generate a starter config in your repository root:

```bash
cmakefmt init
```

Or dump the full default config to stdout:

```bash
cmakefmt config dump > .cmakefmt.yaml
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

## Git Hook

Install a pre-commit hook that checks formatting on every commit:

```bash
cmakefmt install-hook
```

This adds a `git` `pre-commit` hook that runs `cmakefmt --check --staged`.
Commits with unformatted CMake files will be rejected until you run
`cmakefmt --in-place .`.

For `pre-commit` framework integration, see the [CI page](/ci/#pre-commit).

## Getting Help

For a brief summary of all flags:

```bash
cmakefmt -h
```

For an extended description of every flag with examples:

```bash
cmakefmt --help
```

## What's Next

- [Installation](/installation/) — full install options, shell completions, editor setup
- [CLI Reference](/cli/) — every flag documented
- [Config Reference](/config/) — tune `cmakefmt` for your project
- [Formatter Behavior](/behavior/) — understand what gets changed and why
- [Migration From `cmake-format`](/migration/) — coming from the Python tool?
