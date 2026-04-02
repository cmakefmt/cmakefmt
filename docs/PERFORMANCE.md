# Performance

This document describes how to benchmark `cmakefmt`, compare runs over time,
and profile hotspots when the benchmark numbers move in the wrong direction.

## Goals

Phase 8 performance work is trying to answer four questions:

1. How fast is parsing?
2. How fast is formatting from an already-parsed AST?
3. What is the end-to-end user-facing cost of `format_source` and CLI-adjacent paths?
4. How does opt-in parallel execution scale, and what does it cost?

## Benchmark suites

The benchmark target is [benches/formatter.rs](/Users/PuneetMatharu/Dropbox/programming/rust/cmake-format-rust/benches/formatter.rs).

It currently covers:

- `parse/*`
  - parser-only latency
- `format_ast/*`
  - formatter-only cost from a parsed AST
- `format_source/*`
  - end-to-end parser + formatter cost
- `format_source_with_debug/*`
  - debug/barrier overhead
- `file_discovery/*`
  - recursive CMake file discovery
- `config/*`
  - config resolution/loading
- `check_mode/*`
  - check-mode style formatting path
- `in_place_write/*`
  - formatting plus write-path cost
- `batch_format/*`
  - serial vs parallel batch throughput

The workloads include:

- small synthetic sources
- real-world corpus files
- large synthetic stress input (1000+ lines)
- comment-heavy input
- barrier/fence-heavy input

## Running benchmarks

Run the full suite:

```bash
cargo bench --bench formatter
```

Run one benchmark family:

```bash
cargo bench --bench formatter parse/
cargo bench --bench formatter format_source/
```

Save a baseline before a risky change:

```bash
cargo bench --bench formatter -- --save-baseline before-opt
```

Compare against a saved baseline:

```bash
cargo bench --bench formatter -- --baseline before-opt
```

## Profiling

If a benchmark regresses or stays slower than expected, capture a profile.

Example with `cargo flamegraph`:

```bash
cargo flamegraph --bench formatter -- format_source/large_synthetic
```

Useful profiling targets:

- `parse/large_synthetic`
- `format_ast/large_synthetic`
- `format_source/real_world_qtbase_network`
- `format_source_with_debug/barrier_heavy`
- `batch_format/4`

## What to look for

Priority hotspots:

- repeated registry loading or command-spec parsing
- repeated string allocation and cloning
- repeated case normalization on hot paths
- expensive layout decisions in `src/formatter/node.rs`
- barrier/debug bookkeeping overhead
- scaling loss in parallel formatting

When optimizing, keep these invariants intact:

- semantic equivalence
- idempotency
- stable diagnostics
- no loss of comments or disabled barrier regions

## Reporting results

When you change performance-sensitive code, capture:

- the benchmark command you ran
- the baseline comparison if one exists
- the hardware / OS you used
- the hotspot you targeted
- the measured effect

If the change is significant, summarize it in `README.md` as well.

## Current local signal

Current local benchmark signal from this workspace on 2026-04-02:

- `format_source/large_synthetic` (1000+ lines): `8.6841 ms .. 8.9836 ms`

This is useful as a development checkpoint, not a final published result.
The final reported numbers should still include explicit environment details
and a repeatable comparison methodology.
