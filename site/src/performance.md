# Performance

`cmakefmt` is designed to feel fast enough that you do not have to think twice
about using it in local workflows, editor integrations, pre-commit hooks, and CI.

## Current Benchmark Signal

The repository keeps a fuller benchmark record in `docs/PERFORMANCE.md`. The
headline numbers from the current local benchmark set are:

| Metric | Current local signal |
| --- | --- |
| Geometric-mean speedup vs `cmake-format` | `20.77x` |
| Parser-only large synthetic input | `6.9304 ms .. 6.9677 ms` |
| Formatter-only from AST, large synthetic input | `1.5227 ms .. 1.5534 ms` |
| End-to-end `format_source`, large synthetic input | `8.6263 ms .. 8.8934 ms` |
| Debug/barrier-heavy formatting | `315.51 µs .. 319.36 µs` |

## Real-World Comparison

The current local corpus comparison measured `cmakefmt` against `cmake-format`
on a set of real `CMakeLists.txt` files drawn from projects such as:

- Abseil
- Catch2
- CLI11
- GoogleTest
- LLVM
- Qt
- nlohmann/json
- protobuf

On that corpus:

- `cmakefmt` was faster on every fixture
- the speedup ranged from `12.76x` to `50.28x`
- the geometric-mean speedup was `20.77x`

## Parallel Batch Throughput

Multi-file runs stay single-threaded by default, but opt-in parallelism scales
well on the current synthetic whole-corpus benchmark:

| Mode | Time |
| --- | --- |
| serial | `109.5 ms ± 1.3 ms` |
| `--parallel 2` | `71.8 ms ± 1.5 ms` |
| `--parallel 4` | `44.3 ms ± 4.1 ms` |
| `--parallel 8` | `31.9 ms ± 1.0 ms` |

Peak resident memory roughly doubled at `--parallel 8`, which is why the tool
still defaults to single-threaded execution unless you explicitly ask for more
throughput.

## What The Numbers Mean

The important takeaway is not just "small benchmark number good". The important
practical point is that `cmakefmt` is already fast enough for:

- repository-wide `--check` runs in CI
- pre-commit hooks on staged files
- repeated local formatting during development
- editor-triggered formatting on save

## Benchmark Environment

The current headline measurements were captured on:

- macOS `26.3.1`
- `aarch64-apple-darwin`
- `10` logical CPUs
- `rustc 1.94.1`
- `hyperfine 1.20.0`
- `cmake-format 0.6.13`

Exact numbers will vary by machine. What matters release to release is that the
relative trends remain strong and regressions are noticed quickly.

## How To Reproduce

Run the formatter benchmark suite:

```bash
cargo bench --bench formatter
```

Save a baseline before a risky change:

```bash
cargo bench --bench formatter -- --save-baseline before-change
```

Compare a later run to that baseline:

```bash
cargo bench --bench formatter -- --baseline before-change
```

## Related Reading

- [CLI Reference](cli.md)
- [Architecture](architecture.md)
- [Troubleshooting](troubleshooting.md)
