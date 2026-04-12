# Contributing

This repository moves fastest when code, docs, config surface, and tests stay
in sync. If you add a feature or change behavior, update the related files in
the same change.

This applies to both human contributors and any automated tooling used to generate code.

## Dev Environment Setup

The repository ships a [mise](https://mise.jdx.dev) config that installs
every required tool in one step.

Install mise:

```bash
# macOS
brew install mise

# macOS / Linux / WSL
curl https://mise.run | sh
```

Then activate it in your shell (one-time):

```bash
# zsh
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc && source ~/.zshrc

# bash
echo 'eval "$(mise activate bash)"' >> ~/.bashrc && source ~/.bashrc
```

Then from the repo root:

```bash
mise install
```

This installs Rust (stable), Node 22, Python 3.12, `cargo-audit`,
`cargo-deny`, `cargo-llvm-cov`, `wasm-pack`, `pre-commit`, `reuse`, and
`bump-my-version` — and automatically installs the pre-commit and pre-push
hooks as a post-install step.

## Core Rule

When you add a user-visible feature, also update:

- the implementation
- the tests
- the user-facing docs
- `CHANGELOG.md` — add an entry under `## Unreleased`
- the roadmap if the project scope or milestone state changed

Do not leave “I’ll document it later” gaps behind. This repository has many
interconnected components, and documentation that drifts from the implementation
becomes expensive to untangle quickly.

## Changelog

Every user-facing change must have an entry in `CHANGELOG.md` under the
`## Unreleased` section before it is merged. Group entries under `### Added`,
`### Changed`, `### Fixed`, or `### Removed` as appropriate.

Internal-only changes (refactors, CI tweaks, test-only changes) do not need a
changelog entry unless they affect the user experience.

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

Operational-only flags still need:

- docs if they are user-visible
- integration tests in `tests/cli.rs`
- a review of the non-config allowlist in the dump-config guard test in
  `src/main.rs`

## If You Add Or Change A Config Option

Update the runtime config model in `src/config/mod.rs`.

Then update:

- `src/config/file.rs`
  - TOML deserialization structs
  - config merge logic
  - `default_config_template()`
- `src/config/legacy.rs`
  - add a match arm in `merge_format_section` or `merge_markup_section` so
    `--convert-legacy-config` converts the option instead of emitting an
    "unsupported" note
  - add the corresponding field to `OutputFormatSection` / `OutputMarkupSection`
    and its `has_any()` method
- `docs/src/content/docs/config.md`
  - add a subsection to the relevant Options group
  - add the option to the table of contents and the Defaults block
- `README.md`
  - config examples or supported option lists if applicable
- tests covering config parsing, precedence, and behavior

If the option has a CLI override flag, also update:

- `src/main.rs`
- `tests/cli.rs`

Regenerate and commit the JSON schema so that editors with SchemaStore
integration pick up the new option:

```bash
cmakefmt config schema > docs/public/schemas/latest/schema.json
```

The `check-docs` pre-commit hook will fail if the schema is out of date.

The browser playground loads its default config from the WASM
`default_config_yaml()` function at runtime, so config changes are
picked up automatically after a WASM rebuild.

## If You Add An Experimental Option

New formatting behaviors that are not yet ready for stable defaults go in
the `Experimental` struct in `src/config/mod.rs`. Each option must default
to `false` (off) and be gated behind `config.experimental.<option>` in the
formatter.

Then update:

- `src/config/mod.rs`
  - add a `pub` field to `Experimental` (the struct is `#[non_exhaustive]`)
- `src/config/file.rs`
  - `FileConfig` picks up the field automatically via the `experimental`
    section in the config file schema
- `src/main.rs`
  - the `--preview` flag sets all experimental options on; update the
    `build_context` function if the new option needs explicit activation
- `docs/src/content/docs/config.md`
  - document the option under a dedicated Experimental section and mark it
    as unstable
- `CHANGELOG.md`
  - add an entry noting the option is experimental and may change

The promotion path for experimental options:

1. Ship the option behind `[experimental]` for at least one release.
2. Gather feedback — if no issues are reported, promote to a stable default.
3. Promotion means moving the field from `Experimental` to `Config` and
   changing the default to `true`. This is a formatting output change and
   should be documented in the changelog.

## If You Change Formatting Behavior

Update the formatter code under `src/formatter/`.

Then update:

- `tests/snapshots.rs`
- `tests/idempotency.rs`
- any affected fixtures under
  `tests/fixtures/`
- `README.md` if user-visible behavior changed

If the formatted output changes in ways that would affect real-world projects
(different indentation, wrapping behavior, or keyword grouping), review whether
`docs/src/content/docs/architecture.md` should also be updated.

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
- `docs/src/content/docs/performance.md`
- `.github/workflows/ci.yml` if CI should run it
- `.pre-commit-config.yaml` if local hygiene should enforce it
- `README.md` if users should know about it

Release tooling has a split ownership model:

- `.github/workflows/prepare-release.yml`
  - owns version bumps, lockfile refresh, commit creation, tagging, and push
- `.github/workflows/release.yml`
  - builds, verifies, and publishes from an existing tag
- `packaging/homebrew/`
  - stores the Homebrew formula template and notes
  - the rendered formula is generated as a release artifact, not maintained as
    a versioned formula file in-tree

## If You Add Or Change User Docs

Keep these aligned:

- `README.md`
- `docs/`
- `CHANGELOG.md`
- `docs/README.md`
- any relevant published docs pages under `docs/src/content/docs/`

If the change affects contributor workflow or repo structure, also update:

- `src/README.md`
- `tests/README.md`
- `tests/fixtures/README.md`
- `benches/README.md`

If you add a new primary docs page, add it to:

- the sidebar in `docs/astro.config.mjs`
- `docs/src/content/docs/`
- `scripts/check-docs.sh`

The docs site is an Astro + Starlight app. Before you finish a docs-heavy change, run:

```bash
bash scripts/check-docs.sh
```

If you add or edit a before/after formatting example in any doc page, verify
the "after" output by running the input through `cmakefmt` directly — either
via the CLI or the [playground](/playground/). Never write an "after" by hand;
the formatter is the source of truth.

```bash
# Quick way to verify a before/after snippet:
printf 'your_cmake_input_here\n' | cargo run --quiet -- /dev/stdin
```

If you changed the public Rust API or the meaning of exported types/functions,
also update the rustdoc comments on the affected items and verify the API docs
still build:

```bash
cargo doc --no-deps
```

## If You Add Or Edit Licensed Files

Keep REUSE metadata aligned with the change.

- If you add a new prose/config/documentation file that should be covered by
  `REUSE.toml`, add an annotation for its path instead of copying SPDX headers
  into the file unless an inline header is required for that file type.
- If you edit a file in a new calendar year and its copyright year is encoded
  inline or in `REUSE.toml`, update the year so the metadata still matches the
  file history.

Check:

- `REUSE.toml`
- any inline SPDX header on the edited file

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
cargo run -- config dump
```

If you added or changed a config option, also verify the legacy conversion
round-trip produces no new "unsupported" notes:

```bash
cmake-format --dump-config > /tmp/cmf_dump.py
cargo run -- config convert /tmp/cmf_dump.py
```

If you changed docs, also check:

```bash
bash scripts/check-docs.sh
```

## Summary

If you add a feature and do not update tests, docs, and the config template
where relevant, the change is incomplete.
