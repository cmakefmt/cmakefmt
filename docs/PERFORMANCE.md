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

The benchmark target is [benches/formatter.rs](../benches/formatter.rs).

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

The current local numbers below were captured on 2026-04-05 with:

- macOS `26.3.1` on `aarch64-apple-darwin`
- `10` logical CPUs
- `rustc 1.94.1`
- `hyperfine 1.20.0`
- `cmake-format 0.6.13`

## Current Local Signal

Criterion reports `[lower bound, point estimate, upper bound]` for the
95% confidence interval:

- `parse/large_synthetic`: estimate `7.1067 ms` (95% CI `7.0793 ms` to `7.1359 ms`)
- `format_ast/large_synthetic`: estimate `1.7545 ms` (95% CI `1.7425 ms` to `1.7739 ms`)
- `format_source/large_synthetic`: estimate `8.8248 ms` (95% CI `8.8018 ms` to `8.8519 ms`)
- `format_source_with_debug/barrier_heavy`: estimate `313.98 µs` (95% CI `311.89 µs` to `317.54 µs`)

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
python3 scripts/fetch-real-world-corpus.py
hyperfine --warmup 3 --runs 20 \
  "./target/release/cmakefmt 'target/real-world-corpus/<fixture>' >/dev/null" \
  "cmake-format 'target/real-world-corpus/<fixture>' >/dev/null"
```

To generate local before/after review artefacts for the same corpus:

```bash
scripts/review-real-world-corpus.sh
```

Results:

| Fixture | Lines | `cmakefmt` ms | `cmake-format` ms | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `abseil/CMakeLists.txt` | 280 | 5.804 | 168.570 | 29.04x |
| `catch2/CMakeLists.txt` | 230 | 5.768 | 105.614 | 18.31x |
| `cli11/CMakeLists.txt` | 283 | 5.570 | 120.994 | 21.72x |
| `cmake_cmbzip2/CMakeLists.txt` | 25 | 5.042 | 61.751 | 12.25x |
| `googletest/CMakeLists.txt` | 36 | 5.004 | 62.439 | 12.48x |
| `ggml/CMakeLists.txt` | 498 | 7.773 | 210.200 | 27.04x |
| `llama_cpp/CMakeLists.txt` | 286 | 6.257 | 126.584 | 20.23x |
| `llvm_tablegen/CMakeLists.txt` | 83 | 5.172 | 75.429 | 14.58x |
| `nlohmann_json/CMakeLists.txt` | 237 | 5.705 | 138.936 | 24.35x |
| `mariadb_server/CMakeLists.txt` | 656 | 9.774 | 473.879 | 48.49x |
| `opencv_flann/CMakeLists.txt` | 2 | 4.719 | 51.497 | 10.91x |
| `protobuf/CMakeLists.txt` | 351 | 6.226 | 111.802 | 17.96x |
| `spdlog/CMakeLists.txt` | 413 | 9.204 | 213.649 | 23.21x |
| `qtbase_network/CMakeLists.txt` | 420 | 8.146 | 284.355 | 34.91x |

Geometric-mean speedup across the full real-world corpus: `20.69x`.

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

- default single-threaded: `184.5 ms ± 1.3 ms`
- `--parallel 2`: `111.5 ms ± 11.9 ms` (`1.65x` faster than serial)
- `--parallel 4`: `64.7 ms ± 1.1 ms` (`2.85x` faster than serial)
- `--parallel 8`: `48.5 ms ± 1.5 ms` (`3.80x` faster than serial)

Peak memory was measured with `/usr/bin/time -l`.

**RSS (Resident Set Size)** is the peak RAM physically held in memory by the process.
**Peak memory footprint** is the peak total virtual memory committed by the process (a macOS-specific metric).

| Mode | RSS | Peak footprint |
| --- | --- | --- |
| serial | `13.2 MB` | `10.2 MB` |
| `--parallel 8` | `20.7 MB` | `17.8 MB` |

## Phase 12 Large-Repository Survey (`oomph-lib`)

Late-stage parallelism validation was run on a local checkout of:

- repo: `https://github.com/oomph-lib/oomph-lib`
- local path: `/Users/PuneetMatharu/Dropbox/programming/oomph-lib/oomph-lib-repos/forked-oomph-lib`
- discovered CMake files: `612`

Command shape:

```bash
hyperfine --warmup 1 --runs 8 \
  "./target/release/cmakefmt /path/to/oomph-lib >/dev/null" \
  "./target/release/cmakefmt --parallel 2 /path/to/oomph-lib >/dev/null" \
  "./target/release/cmakefmt --parallel 4 /path/to/oomph-lib >/dev/null" \
  "./target/release/cmakefmt --parallel 8 /path/to/oomph-lib >/dev/null"
```

Results:

- default single-threaded: `412.5 ms ± 9.0 ms`
- `--parallel 2`: `296.0 ms ± 3.5 ms` (`1.41x` faster than serial)
- `--parallel 4`: `191.8 ms ± 4.7 ms` (`2.15x` faster than serial)
- `--parallel 8`: `152.5 ms ± 2.8 ms` (`2.71x` faster than serial)

Peak memory with `/usr/bin/time -l`:

| Mode | RSS | Peak footprint |
| --- | --- | --- |
| serial | `11.3 MB` | `8.1 MB` |
| `--parallel 8` | `17.0 MB` | `13.9 MB` |

Direct baseline against `cmake-format` on the same full repository (`612`
discovered files), measured with `/usr/bin/time -l`:

| Tool | Wall time | RSS |
| --- | --- | --- |
| `cmake-format` (sequential) | `45.69 s` | `22.5 MB` |
| `cmakefmt` serial | `0.47 s` | `11.7 MB` (`~97x` faster) |
| `cmakefmt --parallel 8` | `0.19 s` | `17.1 MB` (`~240x` faster) |

Interpretation:

- opt-in parallel mode scales well on the current benchmark corpus
- peak RSS roughly doubles at `--parallel 8` compared to serial, which is
  acceptable for this corpus size, but is why serial remains the default
  until larger-codebase surveys are complete
