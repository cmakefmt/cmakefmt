---
title: Contributing
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Thanks for your interest in contributing to cmakefmt. This page covers the
contributor workflow: setting up your environment, building from source,
running tests, setting up hooks, and keeping docs accurate.

## Feedback and Bug Reports

Bug reports, feature requests, and questions are tracked on GitHub Issues:

**[github.com/cmakefmt/cmakefmt/issues](https://github.com/cmakefmt/cmakefmt/issues)**

When filing a bug, include the output of `cmakefmt --version`, the command you
ran, and a minimal CMake file that reproduces the problem. For formatting
behaviour questions, the [Playground](/playground/) is a quick way to
share a reproducible example.

## Dev Environment Setup

The repository ships a [mise](https://mise.jdx.dev) config that pins and
installs every required tool — Rust, Node, Python, and all cargo/pip tools —
in one step.

Install mise:

```bash
# macOS
brew install mise

# macOS / Linux / WSL
curl https://mise.run | sh
```

Then activate it in your shell (one-time):

```bash
# zsh
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'eval "$(mise activate bash)"' >> ~/.bashrc && source ~/.bashrc
```

Then from the repo root:

```bash
mise install
```

This installs Rust (stable), Node 22, Python 3.12, `cargo-audit`,
`cargo-deny`, `cargo-llvm-cov`, `wasm-pack`, `pre-commit`, `reuse`, and
`bump-my-version` — and automatically installs the pre-commit and pre-push
hooks as a post-install step.

## Building From Source

Clone and build:

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

Hooks are installed automatically by `mise install`. To reinstall manually:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

Useful spot checks before pushing:

```bash
pre-commit run --all-files
cmakefmt --staged --check
```

The hook set covers code-quality checks, security scanning (`cargo audit`,
`cargo deny`), and REUSE/license metadata validation.

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
