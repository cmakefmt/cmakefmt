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

## Helper Scripts

- `run-all-benchmarks.sh` — full per-file + whole-repo + parallelism sweep
  used to regenerate the headline numbers in
  `docs/src/content/docs/performance.mdx`.
- `version-trend-datapoint.sh` — produces the single `(time, binary)`
  measurement that goes into `VersionTrendChart.astro` for a new release.
  Quick: builds nothing, just measures
  `target/release/cmakefmt` against the canonical 656-line
  `mariadb_server/CMakeLists.txt` over 30 runs and prints the literal
  chart-array line to paste in.
- `fetch-repos.sh` — populates `benches/repos/` for the whole-repo and
  parallelism phases of the full sweep.
