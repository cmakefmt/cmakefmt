# Performance

`cmakefmt` is fast enough that you never have to think twice about running it
— in local workflows, editor integrations, pre-commit hooks, or CI. That is
not an accident. Speed is a design goal, not a side effect.

## Current Benchmark Signal

The repository keeps a fuller benchmark record in `docs/PERFORMANCE.md`. The
headline numbers from the current local benchmark set:

| Metric | Current local signal |
| --- | --- |
| Geometric-mean speedup vs `cmake-format` | **`20.77x`** |
| Parser-only, large synthetic input | `6.9304 ms .. 6.9677 ms` |
| Formatter-only from AST, large synthetic input | `1.5227 ms .. 1.5534 ms` |
| End-to-end `format_source`, large synthetic input | `8.6263 ms .. 8.8934 ms` |
| Debug/barrier-heavy formatting | `315.51 µs .. 319.36 µs` |

## Real-World Comparison

The current local corpus comparison measured `cmakefmt` against `cmake-format`
on real `CMakeLists.txt` files drawn from projects including:

- Abseil
- Catch2
- CLI11
- GoogleTest
- LLVM
- Qt
- nlohmann/json
- protobuf

Fetch the pinned local corpus before rerunning those comparisons:

```bash
python3 scripts/fetch-real-world-corpus.py
```

Results across that corpus:

- `cmakefmt` was faster on **every single fixture**
- speedup ranged from `12.76x` to `50.28x`
- geometric-mean speedup: **`20.77x`**

## Parallel Batch Throughput

Multi-file runs are single-threaded by default, but opt-in parallelism scales
well:

| Mode | Time |
| --- | --- |
| serial | `109.5 ms ± 1.3 ms` |
| `--parallel 2` | `71.8 ms ± 1.5 ms` |
| `--parallel 4` | `44.3 ms ± 4.1 ms` |
| `--parallel 8` | **`31.9 ms ± 1.0 ms`** |

Peak resident memory roughly doubles at `--parallel 8`, which is why the tool
defaults to single-threaded execution unless you explicitly ask for more.

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
