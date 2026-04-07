---
title: Contributing
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Thanks for your interest in contributing to cmakefmt. This page covers the
contributor workflow: building from source, running tests, setting up hooks,
and keeping docs accurate.

## Building From Source

You need a Rust toolchain (stable, 1.70+). Clone and build:

```bash
git clone https://github.com/cmakefmt/cmakefmt.git
cd cmakefmt
cargo build
```

Install locally for manual testing:

```bash
cargo install --path .
```

Run the full test suite:

```bash
cargo test
```

## Pre-commit Hooks

The repository ships a `pre-commit` configuration. Install both commit and
pre-push hooks early in your workflow:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Useful spot checks before pushing:

```bash
pre-commit run --all-files
cmakefmt --staged --check
```

The hook set covers code-quality checks and REUSE/license metadata validation.

## Before/After Examples in Docs

When writing or updating documentation that shows a before/after formatting
example, always verify the output using the actual CLI or the
[Playground](/playground/) before committing. Never write an expected "After"
from memory — the formatter's decisions (inline vs. hanging vs. vertical) depend
on line-width and command spec, and can be surprising.

```bash
# Quick verification from the command line
echo 'your_cmake_snippet()' | cmakefmt -
```

## Local Docs Preview

Preview the published docs locally:

```bash
cd docs
npm install
npm run dev
```

Then open the local URL that Astro prints, usually <http://localhost:4321>.

## Changelog

Add an entry to the `## Unreleased` section of `CHANGELOG.md` for any
user-visible change. The docs site changelog is kept in sync automatically by
`scripts/sync-changelog.py`.
