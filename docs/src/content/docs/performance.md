---
title: Performance
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` is fast enough that you never have to think twice about running it
— in local workflows, editor integrations, pre-commit hooks, or CI. That is
not an accident. Speed is a design goal, not a side effect.

## Overview

- **20.69×** geometric-mean speedup over `cmake-format` on real-world CMake files
- **~97×** faster than `cmake-format` on a 612-file repository (sequential)
- **~240×** faster with `--parallel 8` on the same repository
- Fastest individual fixture: **48.49×** speedup (`mariadb_server/CMakeLists.txt`, 656 lines)
- End-to-end format of a 1000+ line synthetic file: **~8.8 ms**

## Benchmark Environment

Current headline measurements were captured on:

- macOS `26.3.1`
- `aarch64-apple-darwin`
- `10` logical CPUs
- `rustc 1.94.1`
- `hyperfine 1.20.0`
- `cmake-format 0.6.13`

Exact numbers vary by machine. What matters across releases is that relative
performance trends remain strong and regressions are caught early.

## Benchmark Results

| Fixture | Lines | `cmakefmt` ms | `cmake-format` ms | Speedup |
|---------------------------------|------:|--------------:|------------------:|--------:|
| `abseil/CMakeLists.txt`         |   280 |         5.804 |           168.570 |  29.04× |
| `catch2/CMakeLists.txt`         |   230 |         5.768 |           105.614 |  18.31× |
| `cli11/CMakeLists.txt`          |   283 |         5.570 |           120.994 |  21.72× |
| `cmake_cmbzip2/CMakeLists.txt`  |    25 |         5.042 |            61.751 |  12.25× |
| `googletest/CMakeLists.txt`     |    36 |         5.004 |            62.439 |  12.48× |
| `ggml/CMakeLists.txt`           |   498 |         7.773 |           210.200 |  27.04× |
| `llama_cpp/CMakeLists.txt`      |   286 |         6.257 |           126.584 |  20.23× |
| `llvm_tablegen/CMakeLists.txt`  |    83 |         5.172 |            75.429 |  14.58× |
| `mariadb_server/CMakeLists.txt` |   656 |         9.774 |           473.879 |  48.49× |
| `nlohmann_json/CMakeLists.txt`  |   237 |         5.705 |           138.936 |  24.35× |
| `opencv_flann/CMakeLists.txt`   |     2 |         4.719 |            51.497 |  10.91× |
| `protobuf/CMakeLists.txt`       |   351 |         6.226 |           111.802 |  17.96× |
| `spdlog/CMakeLists.txt`         |   413 |         9.204 |           213.649 |  23.21× |
| `qtbase_network/CMakeLists.txt` |   420 |         8.146 |           284.355 |  34.91× |

Geometric-mean speedup across the full corpus: **`20.69×`**.
On a 220-file batch, `--parallel 8` improves throughput by **`3.80×`** vs serial.

The following Criterion estimates cover a 1000+ line synthetic stress-test file:

| Metric | Estimate | 95% CI |
| --- | --- | --- |
| Parser-only | `7.1067 ms` | `7.0793–7.1359 ms` |
| Formatter-only (from parsed AST) | `1.7545 ms` | `1.7425–1.7739 ms` |
| End-to-end `format_source` | `8.8248 ms` | `8.8018–8.8519 ms` |
| Debug/barrier-heavy formatting | `313.98 µs` | `311.89–317.54 µs` |

All Criterion estimates show a point estimate with a 95% confidence interval —
the range within which the true mean is expected to fall 95% of the time.
"AST" (Abstract Syntax Tree) is the structured in-memory representation
produced by parsing, before formatting.

## Parallel Batch Throughput

Multi-file runs are single-threaded by default, but opt-in parallelism scales
well:

| Mode | Time |
| --- | --- |
| serial | `184.5 ms ± 1.3 ms` |
| `--parallel 2` | `111.5 ms ± 11.9 ms` |
| `--parallel 4` | `64.7 ms ± 1.1 ms` |
| `--parallel 8` | **`48.5 ms ± 1.5 ms`** |

Peak RSS (Resident Set Size — the RAM physically held in memory by the process)
rises from `13.2 MB` (serial) to `20.7 MB` (`--parallel 8`) on this batch. That
is why the tool defaults to single-threaded execution unless you explicitly ask
for more.

## Large Repository Parallelism Survey

Parallelism scaling was also measured on a larger real-world repository
containing `612` discovered CMake files:

| Mode | Time |
| --- | --- |
| serial | `412.5 ms ± 9.0 ms` |
| `--parallel 2` | `296.0 ms ± 3.5 ms` |
| `--parallel 4` | `191.8 ms ± 4.7 ms` |
| `--parallel 8` | **`152.5 ms ± 2.8 ms`** |

That corresponds to a `2.71x` speedup at `--parallel 8` versus serial, with
peak RSS moving from `11.3 MB` to `17.0 MB`.

A direct head-to-head comparison against `cmake-format` on the same `612`-file
tree (`/usr/bin/time -l`) showed:

- `cmake-format` (sequential): `45.69 s` real
- `cmakefmt` serial: `0.47 s` real (`~97×` faster)
- `cmakefmt --parallel 8`: `0.19 s` real (`~240×` faster)

## What The Numbers Mean In Practice

The headline numbers matter not as abstract benchmarks, but because they change
what feels viable:

- repository-wide `--check` in CI — **comfortable**
- pre-commit hooks on staged files — **instant**
- repeated local formatting during development — **no delay you will notice**
- editor-triggered format-on-save — **faster than the save dialog**

## How To Reproduce

Run the formatter benchmark suite:

```bash
cargo bench --bench formatter
```

Save a baseline before a risky change:

```bash
cargo bench --bench formatter -- --save-baseline before-change
```

Compare a later run against that baseline:

```bash
cargo bench --bench formatter -- --baseline before-change
```

## Related Reading

- [CLI Reference](/cli/)
- [Architecture](/architecture/)
- [Troubleshooting](/troubleshooting/)
