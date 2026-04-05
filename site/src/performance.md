# Performance

`cmakefmt` is fast enough that you never have to think twice about running it
— in local workflows, editor integrations, pre-commit hooks, or CI. That is
not an accident. Speed is a design goal, not a side effect.

## Current Benchmark Signal

The repository keeps a fuller benchmark record in `docs/PERFORMANCE.md`. The
headline numbers from the current local benchmark set:

| Metric | Current local signal |
| --- | --- |
| Geometric-mean speedup vs `cmake-format` | **`20.69x`** |
| Parser-only, large synthetic input (1000+ lines) | estimate `7.1067 ms` (95% CI `7.0793–7.1359 ms`) |
| Formatter-only from parsed AST, large synthetic input | estimate `1.7545 ms` (95% CI `1.7425–1.7739 ms`) |
| End-to-end `format_source`, large synthetic input | estimate `8.8248 ms` (95% CI `8.8018–8.8519 ms`) |
| Debug/barrier-heavy formatting | estimate `313.98 µs` (95% CI `311.89–317.54 µs`) |

All Criterion estimates show a point estimate with a 95% confidence interval —
the range within which the true mean is expected to fall 95% of the time.
"Large synthetic input" refers to a 1000+ line stress-test `CMakeLists.txt`
generated for benchmarking purposes. "AST" (Abstract Syntax Tree) is the
structured in-memory representation produced by parsing, before formatting.

## Real-World Comparison

The current local corpus comparison measured `cmakefmt` against `cmake-format`
on real `CMakeLists.txt` files drawn from projects including:

- Abseil
- Catch2
- CLI11
- GoogleTest
- ggml
- llama.cpp
- MariaDB Server
- LLVM
- Qt
- nlohmann/json
- protobuf
- spdlog

Fetch the pinned local corpus before rerunning those comparisons:

```bash
python3 scripts/fetch-real-world-corpus.py
```

Results across that corpus:

- `cmakefmt` was faster on **every single fixture**
- speedup ranged from `10.91x` to `48.49x`
- geometric-mean speedup: **`20.69x`**

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

Phase 12 validation was also run against `oomph-lib` (local checkout with `612`
discovered CMake files):

| Mode | Time |
| --- | --- |
| serial | `412.5 ms ± 9.0 ms` |
| `--parallel 2` | `296.0 ms ± 3.5 ms` |
| `--parallel 4` | `191.8 ms ± 4.7 ms` |
| `--parallel 8` | **`152.5 ms ± 2.8 ms`** |

That corresponds to a `2.71x` speedup at `--parallel 8` versus serial, with
peak RSS moving from `11.3 MB` to `17.0 MB`.

For a direct tool baseline on the same full `oomph-lib` tree (`612` discovered
files), `/usr/bin/time -l` measured:

- `cmake-format` (sequential over discovered files): `45.69 s` real
- `cmakefmt` serial: `0.47 s` real (`~97x` faster)
- `cmakefmt --parallel 8`: `0.19 s` real (`~240x` faster)

## What The Numbers Mean In Practice

The headline numbers matter not as abstract benchmarks, but because they change
what feels viable:

- repository-wide `--check` in CI — **comfortable**
- pre-commit hooks on staged files — **instant**
- repeated local formatting during development — **no delay you will notice**
- editor-triggered format-on-save — **faster than the save dialog**

## Benchmark Environment

Current headline measurements were captured on:

- macOS `26.3.1`
- `aarch64-apple-darwin`
- `10` logical CPUs
- `rustc 1.94.1`
- `hyperfine 1.20.0`
- `cmake-format 0.6.13`

Exact numbers vary by machine. What matters release to release is that
relative trends stay strong and regressions are caught quickly.

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

- [CLI Reference](cli.md)
- [Architecture](architecture.md)
- [Troubleshooting](troubleshooting.md)
