---
title: Getting Started
description: Install cmakefmt and format your first CMake file in under a minute.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` is a fast, workflow-first formatter for CMake files. This page takes you from
zero to a formatted repository in under a minute.

## Install

**Homebrew** is the recommended path for macOS and Linux users — no Rust toolchain needed:

```bash
brew install cmakefmt/cmakefmt/cmakefmt
```

**Cargo** for developers already using Rust:

```bash
cargo install cmakefmt-rust
```

Verify the install:

```bash
cmakefmt --version
```

For Windows, pre-built binaries, and more install options see the full [Install](/install/) page.

## Generate a Config

Dump a starter config to your repository root:

```bash
cmakefmt --dump-config > .cmakefmt.yaml
```

YAML is the recommended format. Open the file and adjust `format.line_width`,
`style.command_case`, or any other option that doesn't match your project's conventions.
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

- [Install](/install/) — full install options, shell completions, editor setup
- [CLI Reference](/cli/) — every flag documented
- [Config Reference](/config/) — tune `cmakefmt` for your project
- [Formatter Behavior](/behavior/) — understand what gets changed and why
- [Migration From `cmake-format`](/migration/) — coming from the Python tool?
