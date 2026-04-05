# `tests/`

This directory contains integration coverage for the crate and CLI.

## Test Modules

- `cli.rs`
  - end-to-end CLI behavior and exit-code coverage
- `idempotency.rs`
  - ensures formatting is stable and preserves parse-tree semantics
- `parser_fixtures.rs`
  - ensures all parser fixtures continue to parse
- `real_world.rs`
  - validates the fetched real-world corpus manifest and local idempotency
- `snapshots.rs`
  - focused formatting behavior regressions

## Supporting Data

- `fixtures/`
  - source inputs grouped by purpose
- `snapshots/`
  - expected formatted outputs for snapshot-backed tests

When adding a new behavior regression, prefer:

1. a focused snapshot in `snapshots.rs`
2. a fixture if the input is reused
3. a real-world corpus manifest or review update only when the issue is genuinely corpus-shaped
