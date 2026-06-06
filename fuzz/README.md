<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# cmakefmt fuzzing harness

This directory hosts a [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html)
project that drives the cmakefmt parser and `parse → format → parse`
round-trip with coverage-guided random inputs.

## Why a separate crate

`cargo-fuzz` requires nightly Rust and emits sanitizer build flags
(`-Zsanitizer=address`, `libfuzzer-sys` link shims, etc.) that would
contaminate the stable build of the parent crate. The `fuzz/` directory
is therefore deliberately **not** a member of the main workspace. The
empty `[workspace]` table in `fuzz/Cargo.toml` detaches it from any
ancestor workspace and is the standard cargo-fuzz pattern.

## Targets

| Target | What it exercises |
| --- | --- |
| `parse` | `cmakefmt::parser::parse` on arbitrary bytes (decoded with `String::from_utf8_lossy`). |
| `format_roundtrip` | `format_source` on arbitrary UTF-8, then asserts three guarantees on any input the formatter accepts: **no panics**, **semantic preservation** (`cmakefmt::semantic::semantic_equivalent` — the same checker behind `--verify` — confirms the formatted output has the same commands and arguments, ignoring comments/whitespace/case), and **idempotency** (`format(format(x)) == format(x)`). `semantic_equivalent` returns `true` when either side fails to parse, so it never false-positives on inputs the parser rejects. |

A small seed corpus lives at `corpus/parse/` and `corpus/format_roundtrip/`,
copied from `tests/fixtures/basic/` and `tests/fixtures/comments/`.
libFuzzer mutates and extends this corpus at runtime; the generated
inputs are git-ignored.

## Running locally

```sh
cargo +nightly fuzz run parse
cargo +nightly fuzz run format_roundtrip
```

Press `Ctrl-C` to stop. To bound the run (e.g. for a quick smoke test):

```sh
cargo +nightly fuzz run parse -- -max_total_time=60
```

## CI cadence

The [`Fuzz`](../.github/workflows/fuzz.yml) workflow runs each target
for **five minutes** weekly (Saturday 04:00 UTC) and on manual
`workflow_dispatch`. It is **advisory only** (`continue-on-error: true`)
and never blocks merges; the goal is long-horizon reassurance, not a
required gate. After a stabilisation period the gate can be promoted to
required by removing the `continue-on-error` flags.

## Triaging a crash artifact

If CI finds a crash it uploads the offending input under
`fuzz-artifacts-<target>`. To reproduce locally:

```sh
# Reproduce the crash
cargo +nightly fuzz run <target> path/to/crash-<sha>

# Pretty-print the offending input for the bug report
cargo +nightly fuzz fmt <target> path/to/crash-<sha>
```

File a bug report including the formatted input and the panic
back-trace.
