<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `benches/`

This directory contains Criterion benchmarks for `cmakefmt`.

## Scope

The current suite measures:

- parser-only cost
- formatter-only cost from a parsed AST
- end-to-end formatting
- debug/barrier overhead
- config loading
- file discovery
- check-mode and write-path costs
- batch throughput and parallel scaling

## When To Update It

Update the benchmark suite when:

- a new major capability is added to the CLI or library
- a hot path changes significantly
- a new benchmark corpus shape is needed
- methodology in `docs/src/performance.md` changes

Keep benchmark names stable unless there is a strong reason to rename them.
