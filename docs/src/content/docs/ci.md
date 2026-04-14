---
title: CI Integration
description: Run `cmakefmt` in GitHub Actions, GitLab CI, and pre-commit.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` is designed to be a zero-friction CI step. Use `--check` to fail the
build when files are not formatted, or `--in-place` to auto-format and commit
the result.

## GitHub Actions

The official `cmakefmt/cmakefmt-action` installs the right binary for the
runner OS and adds it to `PATH`. No Rust toolchain required.

### Check mode (recommended)

Fail the workflow if any CMake files are not formatted:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    args: '--check .'
```

### Auto-format

Format files in-place. Combine with a commit step to push the changes
automatically:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    args: '--in-place .'
```

### Install only

Install `cmakefmt` without running it, then call it yourself with custom flags:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    args: ''

- run: cmakefmt --check --report-format github .
```

### Pin a specific version

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    version: '0.2.0'
    args: '--check .'
```

### Full example

```yaml
name: Format check

on: [push, pull_request]

jobs:
  cmakefmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd  # v6.0.2
      - uses: cmakefmt/cmakefmt-action@v2
        with:
          args: '--check .'
```

## GitLab CI

Download the pre-built Linux binary directly from GitHub Releases:

```yaml
cmakefmt:
  stage: lint
  image: ubuntu:latest
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl
    - |
      LATEST=$(curl -sI https://github.com/cmakefmt/cmakefmt/releases/latest \
        | grep -i '^location:' \
        | sed 's|.*/tag/v||;s/[[:space:]]//g')
      curl -sSL \
        "https://github.com/cmakefmt/cmakefmt/releases/download/v${LATEST}/cmakefmt-x86_64-unknown-linux-musl.tar.gz" \
        | tar -xz -C /usr/local/bin
  script:
    - cmakefmt --check .
```

Or install via Cargo if you already have a Rust image:

```yaml
cmakefmt:
  stage: lint
  image: rust:latest
  cache:
    paths:
      - $CARGO_HOME/bin/
  script:
    - cargo install cmakefmt-rust --quiet
    - cmakefmt --check .
```

## Azure Pipelines

```yaml
steps:
  - script: |
      LATEST=$(curl -sI https://github.com/cmakefmt/cmakefmt/releases/latest \
        | grep -i '^location:' \
        | sed 's|.*/tag/v||;s/[[:space:]]//g')
      curl -sSL \
        "https://github.com/cmakefmt/cmakefmt/releases/download/v${LATEST}/cmakefmt-${LATEST}-x86_64-unknown-linux-musl.tar.gz" \
        | tar -xz --strip-components=1 -C /usr/local/bin
    displayName: Install cmakefmt
  - script: cmakefmt --check .
    displayName: Check CMake formatting
```

Or with Cargo:

```yaml
steps:
  - script: cargo install cmakefmt-rust --quiet
    displayName: Install cmakefmt
  - script: cmakefmt --check .
    displayName: Check CMake formatting
```

## Bitbucket Pipelines

```yaml
pipelines:
  default:
    - step:
        name: Check CMake formatting
        image: rust:latest
        caches:
          - cargo
        script:
          - cargo install cmakefmt-rust --quiet
          - cmakefmt --check .
```

## Docker

Pre-built images are published to GitHub Container Registry on every release:

```bash
docker run --rm -v "$(pwd):/work" -w /work ghcr.io/cmakefmt/cmakefmt --check .
```

Or format a single file via stdin:

```bash
cat CMakeLists.txt | docker run --rm -i ghcr.io/cmakefmt/cmakefmt -
```

Pin a specific version:

```bash
docker run --rm -v "$(pwd):/work" -w /work ghcr.io/cmakefmt/cmakefmt:0.4.0 --check .
```

You can also build the image locally from the repository root:

```bash
docker build -t cmakefmt .
```

## pre-commit

Add a local hook to your `.pre-commit-config.yaml`. This runs `cmakefmt --check`
on every staged CMake file before the commit is created:

```yaml
repos:
  - repo: local
    hooks:
      - id: cmakefmt
        name: cmakefmt
        language: system
        entry: cmakefmt --check
        files: '(CMakeLists\.txt|\.cmake)$'
        pass_filenames: true
```

Install the hook once per clone:

```bash
pre-commit install
```

You can also run it manually across all files:

```bash
pre-commit run cmakefmt --all-files
```

## Check only changed files

When working on large repositories, limit formatting checks to files changed
since a given ref to keep CI fast:

```bash
# Check files changed since the last tag
cmakefmt --check --changed v0.1.0

# Check only staged files (useful locally)
cmakefmt --check --staged
```
