# Contributing

This repository moves fastest when code, docs, config surface, and tests stay
in sync. If you add a feature or change behavior, update the related files in
the same change.

This applies to both human contributors and Codex.

## Core Rule

When you add a user-visible feature, also update:

- the implementation
- the tests
- the user-facing docs
- the roadmap if the project scope or milestone state changed

Do not leave “I’ll document it later” gaps behind. This repo already has enough
moving parts that drift becomes expensive quickly.

## If You Add Or Change A CLI Flag

Update the CLI implementation in `src/main.rs`.

Then update:

- `README.md`
  - usage examples
  - the CLI flag list
  - exit code notes if behavior changed
- `tests/cli.rs`
  - add or update integration coverage for the new behavior

If the new flag is a config-backed override or exposes a formatter/config
feature, also update:

- `src/config/file.rs`
  - `default_config_template()`

The dump-config guard test in `src/main.rs` expects every config-backed long
flag to appear in the dumped config template. If a new long flag is
intentionally operational-only, add it to the non-config allowlist in that
test.

## If You Add Or Change A Config Option

Update the runtime config model in `src/config/mod.rs`.

Then update:

- `src/config/file.rs`
  - TOML deserialization structs
  - config merge logic
  - `default_config_template()`
- `README.md`
  - config examples
  - supported option lists
- tests covering config parsing, precedence, and behavior

If the option has a CLI override flag, also update:

- `src/main.rs`
- `tests/cli.rs`

## If You Change Formatting Behavior

Update the formatter code under `src/formatter/`.

Then update:

- `tests/snapshots.rs`
- `tests/idempotency.rs`
- any affected fixtures under
  `tests/fixtures/`
- `README.md` if user-visible behavior changed

If formatting semantics changed in a meaningful way, review whether
`docs/src/architecture.md`
should also be updated.

If the change affects real-world behavior, also update:

- `tests/fixtures/real_world/`
  - add, remove, or refresh corpus files intentionally
- `tests/fixtures/real_world/SOURCES.md`
  - keep the fixture manifest and provenance links in sync
- `tests/real_world.rs`
  - update coverage rules if the corpus shape changes
- `tests/snapshots/`
  - refresh real-world snapshots when formatted output changes

## If You Change Parsing Or Specs

Parser changes usually require coordinated updates across:

- `src/parser/cmake.pest`
- `src/parser/mod.rs`
- `src/parser/ast.rs`
- parser fixture tests and any snapshots affected by parse-tree changes

Spec or built-in command changes usually require coordinated updates across:

- `src/spec/mod.rs`
- `src/spec/registry.rs`
- `src/spec/builtins.toml`
- spec registry tests

When you update the built-in spec to a newer upstream CMake release, also:

- bump `[metadata].cmake_version` and `[metadata].audited_at` in `src/spec/builtins.toml`
- update the audited version mention in `README.md`
- add or adjust registry tests for the new commands, forms, or keywords
- use the official CMake command docs and release notes as the source of truth

Do not claim support for a newer audited CMake spec version unless the built-in
registry and tests were updated together.

## If You Add Benchmarks Or Tooling

Keep these aligned:

- `benches/`
- `docs/PERFORMANCE.md`
- `.github/workflows/ci.yml` if CI should run it
- `.pre-commit-config.yaml` if local hygiene should enforce it
- `README.md` if users should know about it

## If You Add Or Change User Docs

Keep these aligned:

- `README.md`
- `docs/`
- `CHANGELOG.md`
- `docs/README.md`
- any relevant long-form docs under `docs/src/`

If the change affects contributor workflow or repo structure, also update:

- `src/README.md`
- `tests/README.md`
- `tests/fixtures/README.md`
- `benches/README.md`

If you add a new primary docs page, add it to:

- `docs/src/SUMMARY.md`
- `docs/src/README.md` or another appropriate chapter
- `scripts/check-docs.sh`

The docs site is an `mdBook`. Before you finish a docs-heavy change, run:

```bash
bash scripts/check-docs.sh
```

If you changed the public Rust API or the meaning of exported types/functions,
also update the rustdoc comments on the affected items and verify the API docs
still build:

```bash
cargo doc --no-deps
```

## Before You Finish A Change

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

If you changed CLI behavior, also check:

```bash
cargo run -- --help
cargo run -- --dump-config
```

If you changed docs, also check:

```bash
bash scripts/check-docs.sh
```

## Summary

If you add a feature and do not update tests, docs, and the config template
where relevant, the change is incomplete.
