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

The official [`cmakefmt/cmakefmt-action`](https://github.com/cmakefmt/cmakefmt-action)
installs the correct binary for the runner OS, adds it to `PATH`, and runs
`cmakefmt` with sensible defaults. No Rust toolchain or Python environment is
required on the runner.

The action exposes two high-level inputs that cover almost every workflow:

- `mode` selects what `cmakefmt` does: `check` (verify formatting), `diff`
  (verify and print a unified diff), `fix` (reformat in place), or `setup`
  (install only, do not run).
- `scope` selects which files are processed: `all` (the whole repository),
  `changed` (files modified since the base ref), or `staged` (Git-tracked
  files in the index).

The `paths`, `since`, `version`, and `working-directory` inputs cover the rest.
Reach for `args` only when you need flags none of the structured inputs
expose.

### Strict whole-repo check (recommended starting point)

The default behaviour: run `cmakefmt --check --report-format github .` over
the entire repository. The job fails on the first file that would change, and
inline annotations on the pull request show exactly where.

```yaml
- uses: actions/checkout@v6
- uses: cmakefmt/cmakefmt-action@v2
```

### Changed-file check for incremental rollout

When adopting `cmakefmt` in an existing repository without reformatting every
CMake file on day one, scope the check to files modified since the base ref:

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0   # required so the action can resolve the base ref
- uses: cmakefmt/cmakefmt-action@v2
  with:
    mode: diff
    scope: changed
```

On pull requests, `scope: changed` compares against
`origin/${{ github.base_ref }}`. On push events, it compares against the
push event's `before` commit. The `since` input lets you override the base
ref when you want to compare against, for example, the most recent tag.

### Auto-format and commit

Reformat files on the runner and push the result back. Combine `mode: fix`
with a commit step:

```yaml
- uses: actions/checkout@v6
- uses: cmakefmt/cmakefmt-action@v2
  with:
    mode: fix
- name: Commit any reformat
  uses: stefanzweifel/git-auto-commit-action@v5
  with:
    commit_message: "chore: auto-format CMake files"
```

### Install only, run yourself

When you want full control over the `cmakefmt` invocation, install the
binary with `mode: setup` and run it directly:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    mode: setup
- run: cmakefmt --check --report-format sarif . > cmakefmt.sarif
```

### Pin a specific version

The action installs the latest release by default. To pin to a specific
version (e.g. for reproducible CI on long-lived branches):

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    version: "1.3.0"
```

### Monorepo subdirectory

When CMake files live under a subdirectory rather than the repository root,
point the action at it with `working-directory`:

```yaml
- uses: cmakefmt/cmakefmt-action@v2
  with:
    working-directory: cpp/
```

### Full example

```yaml
name: Format check

on: [push, pull_request]

jobs:
  cmakefmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: cmakefmt/cmakefmt-action@v2
```

The complete list of inputs and outputs is documented in the
[`cmakefmt-action` README](https://github.com/cmakefmt/cmakefmt-action#readme).

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
docker run --rm -v "$(pwd):/work" -w /work ghcr.io/cmakefmt/cmakefmt:1.3.0 --check .
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
