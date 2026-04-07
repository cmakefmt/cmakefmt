---
title: Coverage
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` treats coverage as a contributor tool, not as a vanity number.

The goal is simple: make it obvious which parts of the formatter are exercised
by the default test suite, and make it easy to inspect the results locally and
in CI.

## What The Coverage Workflow Runs

GitHub Actions runs coverage with `cargo llvm-cov` on the default test suite:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-targets --summary-only
cargo llvm-cov report --workspace --all-targets --html
```

That means coverage includes:

- library code under `src/`
- the CLI binary under `src/main.rs`
- unit tests
- integration tests

The workflow publishes:

- a text summary in the GitHub Actions job summary
- the raw summary as an artifact
- an HTML report as an artifact for line-by-line inspection

## What Coverage Is Not Trying To Measure

Coverage is helpful, but it is not the whole quality story. `cmakefmt` still
leans heavily on:

- snapshot tests for formatter behavior
- idempotency checks
- real-world corpus tests
- performance benchmarks

High code coverage with weak real-world corpus coverage provides false confidence — both matter.

## Local Coverage

Install `cargo-llvm-cov` once:

```bash
cargo install cargo-llvm-cov
```

Then run coverage locally:

```bash
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-targets --summary-only
cargo llvm-cov report --workspace --all-targets --html
```

The HTML report is written under `target/llvm-cov/html/`.

## Reading The Results

When coverage changes, pay attention to where the delta lands:

- parser and formatter core paths matter more than trivial getters
- config discovery and CLI integration matter because they are user-facing
- new CLI features should ship with direct integration coverage
- performance-sensitive hot paths should still keep behavior tests around them

In short: coverage is a guide to missing tests, not a substitute for good test
design.
