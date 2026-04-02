# Performance

This document describes how to benchmark `cmakefmt`, compare runs over time,
profile hotspots, and interpret the current local performance signal.

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

Equivalent profilers are also acceptable when `cargo-flamegraph` is not
available. On macOS, `sample` is sufficient for a call-tree pass:

```bash
./target/release/cmakefmt /tmp/cmakefmt-batch-big >/dev/null &
pid=$!
sample "$pid" 2 1 -mayDie > /tmp/cmakefmt.sample.txt
wait "$pid"
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

## Benchmark Environment

The current local numbers below were captured on 2026-04-02 with:

- macOS `26.3.1` on `aarch64-apple-darwin`
- `10` logical CPUs
- `rustc 1.94.1`
- `hyperfine 1.20.0`
- `cmake-format 0.6.13`

## Current Local Signal

Criterion:

- `parse/large_synthetic`: `6.9304 ms .. 6.9677 ms`
- `format_ast/large_synthetic`: `1.5227 ms .. 1.5534 ms`
- `format_source/large_synthetic`: `8.6263 ms .. 8.8934 ms`
- `format_source_with_debug/barrier_heavy`: `315.51 µs .. 319.36 µs`

Interpretation:

- end-to-end large-file formatting remains under the `< 10 ms` Phase 8 target
- parsing plus AST construction is still the dominant cost, roughly `79%` of
  end-to-end large-file latency on the synthetic 1000+ line workload
- formatter-only work is materially smaller, roughly `17%` to `18%` of the
  same workload
- debug/barrier bookkeeping is visible but small on its dedicated workload

## Profiling Results

An equivalent profiler pass was captured with macOS `sample` on a large
synthetic batch corpus (`/tmp/cmakefmt-batch-big`, `2200` files).

The useful top-of-stack summary from that sample was:

- `921` total sampled stacks in the main thread
- `800` stacks under `cmakefmt::process_target`
- `782` stacks under `cmakefmt::formatter::format_source_impl`
- `619` stacks under `cmakefmt::formatter::flush_enabled_chunk`
- `560` stacks under `cmakefmt::parser::parse`
- `539` stacks inside `pest::parser_state::state`
- `141` stacks in `cmakefmt::formatter::format_file`
- `14` stacks in `cmakefmt::formatter::node::format_command`

What that means:

- the parser remains the hottest part of the end-to-end path
- the dominant parser hotspot is still the generated `pest` state machine,
  especially unquoted-argument parsing
- formatter layout code is hot enough to matter, but it is not the primary
  bottleneck right now
- command registry lookup no longer shows up as a primary cost after the
  registry caching work

The concrete optimizations already in place that this profiling validates are:

- built-in command registry caching via `CommandRegistry::builtins()`
- avoiding extra line-splitting allocation in the barrier scan path

The next meaningful speedups are therefore more likely to come from reducing
parser work than from micro-optimizing command lookup.

## `cmakefmt` vs `cmake-format`

The real-world corpus comparison was measured with:

```bash
hyperfine --warmup 3 --runs 20 \
  "./target/release/cmakefmt 'tests/fixtures/real_world/<fixture>' >/dev/null" \
  "cmake-format 'tests/fixtures/real_world/<fixture>' >/dev/null"
```

Results:

| Fixture | Lines | `cmakefmt` ms | `cmake-format` ms | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `abseil/CMakeLists.txt` | 204 | 4.467 | 114.576 | 25.65x |
| `catch2/CMakeLists.txt` | 231 | 4.558 | 101.606 | 22.29x |
| `cli11/CMakeLists.txt` | 283 | 4.458 | 118.954 | 26.68x |
| `cmake_cmbzip2/CMakeLists.txt` | 25 | 3.957 | 59.156 | 14.95x |
| `googletest/CMakeLists.txt` | 36 | 4.138 | 60.558 | 14.64x |
| `llvm_tablegen/CMakeLists.txt` | 83 | 4.257 | 73.627 | 17.30x |
| `monorepo_root.cmake` | 40 | 4.330 | 69.929 | 16.15x |
| `nlohmann_json/CMakeLists.txt` | 237 | 4.717 | 131.813 | 27.95x |
| `opencv_flann/CMakeLists.txt` | 2 | 3.899 | 49.754 | 12.76x |
| `protobuf/CMakeLists.txt` | 201 | 4.631 | 85.811 | 18.53x |
| `qtbase_network/CMakeLists.txt` | 420 | 5.557 | 279.420 | 50.28x |

Geometric-mean speedup across the full real-world corpus: `20.77x`.

Notes:

- `cmakefmt` was faster on every real-world fixture in the current corpus
- the very smallest fixtures are partially dominated by shell startup overhead,
  so the tiny-file numbers are more conservative than a pure in-process
  library benchmark

## Parallel Throughput and Memory

Whole-corpus throughput on a synthetic batch directory with `220` files:

```bash
hyperfine --warmup 3 --runs 15 \
  "./target/release/cmakefmt /tmp/cmakefmt-batch >/dev/null" \
  "./target/release/cmakefmt --parallel 2 /tmp/cmakefmt-batch >/dev/null" \
  "./target/release/cmakefmt --parallel 4 /tmp/cmakefmt-batch >/dev/null" \
  "./target/release/cmakefmt --parallel 8 /tmp/cmakefmt-batch >/dev/null"
```

Results:

- default single-threaded: `109.5 ms ± 1.3 ms`
- `--parallel 2`: `71.8 ms ± 1.5 ms` (`1.53x` faster than serial)
- `--parallel 4`: `44.3 ms ± 4.1 ms` (`2.47x` faster than serial)
- `--parallel 8`: `31.9 ms ± 1.0 ms` (`3.43x` faster than serial)

Peak memory was measured with `/usr/bin/time -l`:

- serial: `8159232` max resident set size, `6685568` peak memory footprint
- `--parallel 8`: `15876096` max resident set size, `14418880` peak memory footprint

Interpretation:

- opt-in parallel mode scales well on the current benchmark corpus
- peak RSS is roughly doubled at `--parallel 8`, which is acceptable for this
  corpus size but still justifies keeping the default execution mode
  single-threaded until larger-codebase surveys are complete
