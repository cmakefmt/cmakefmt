# cmakefmt — Project Guide

`cmakefmt` — a Rust reimplementation of `cmake-format` (from the dead `cmakelang` Python project).
Goal: fast, correct, configurable CMake formatter distributed as a single binary.

## Key decisions (do not revisit without good reason)

- **Parser**: `pest` (PEG, pure Rust). Grammar lives in `src/parser/cmake.pest`.
- **Formatter**: Wadler-Lindig algorithm via the `pretty` crate. AST → Doc IR → string.
- **Config**: YAML/TOML via `serde`. Config files: `.cmakefmt.yaml`, `.cmakefmt.yml`, `.cmakefmt.toml`.
- **CLI**: `clap` (derive API).
- **Snapshot tests**: `insta` crate. Snapshots live in `tests/snapshots/`.
- **Idempotency invariant**: `format(format(x)) == format(x)` must always hold.

## Build & run

```bash
cargo build                        # debug build
cargo build --release              # release build
cargo run -- path/to/CMakeLists.txt
cargo run -- --check path/to/CMakeLists.txt
```

## Testing

```bash
cargo test                         # all tests
cargo test parser                  # parser unit tests
cargo test formatter               # formatter unit tests
cargo test --test snapshots        # snapshot tests only
cargo test --test cli              # CLI integration tests
cargo test --test real_world       # real-world corpus tests
cargo insta review                 # review snapshot changes interactively
```

Idempotency tests run automatically as part of `cargo test`.

## Linting & formatting

```bash
cargo clippy -- -D warnings        # must pass clean
cargo fmt --check                  # must pass clean
```

CI runs both of these — fix before committing.

## Project layout

```
cmakefmt/
├── AGENTS.md                  ← this file
├── Cargo.toml
├── docs/
│   ├── README.md              ← docs site contributor notes
│   ├── logo.png               ← square docs/site logo source
│   ├── astro.config.mjs       ← Astro + Starlight site configuration
│   ├── package.json           ← docs app dependencies and scripts
│   ├── public/                ← static site assets such as robots.txt
│   └── src/
│       ├── content/docs/      ← published docs pages
│       ├── assets/            ← docs site branding assets
│       └── styles/            ← Starlight theme customizations
├── src/
│   ├── main.rs                ← CLI entry point (clap)
│   ├── lib.rs                 ← public API (format_source, format_file)
│   ├── files.rs               ← CMake file discovery, .cmakefmtignore support
│   ├── config/
│   │   ├── mod.rs             ← Config struct, defaults, resolution logic
│   │   ├── file.rs            ← TOML config file loading + merging
│   │   └── legacy.rs          ← conversion from legacy cmake-format config files
│   ├── parser/
│   │   ├── mod.rs             ← public parse() fn, error types
│   │   ├── cmake.pest         ← pest grammar (THE source of truth)
│   │   └── ast.rs             ← CST → AST conversion, AST node types
│   ├── spec/
│   │   ├── mod.rs             ← NArgs, PosSpec, KwargSpec, CommandForm, CommandSpec
│   │   ├── registry.rs        ← CommandRegistry: loads builtins + user overrides
│   │   └── builtins.toml      ← built-in specs for all ~150 CMake commands
│   ├── formatter/
│   │   ├── mod.rs             ← public format() fn
│   │   ├── node.rs            ← AST node → Doc IR conversion (uses CommandRegistry)
│   │   └── comment.rs         ← comment attachment + preservation logic
│   └── error.rs               ← unified error type (thiserror)
├── tests/
│   ├── snapshots/             ← insta snapshot files (committed to repo)
│   ├── cli.rs                 ← CLI integration tests (stdout, stdin, color, --check)
│   ├── idempotency.rs         ← format(format(x)) == format(x)
│   ├── parser_fixtures.rs     ← parser unit tests driven by fixture files
│   ├── real_world.rs          ← formatting tests against real-world corpus
│   ├── snapshots.rs           ← snapshot-based formatting tests
│   └── fixtures/              ← .cmake input files for tests
│       ├── basic/
│       ├── comments/
│       ├── edge_cases/
│       └── real_world/        ← real CMakeLists.txt files from open-source projects
└── benches/
    └── formatter.rs           ← criterion benchmarks
```

## Comment handling

Comments are first-class AST nodes — they are NOT stripped and reattached.
The pest grammar captures them inline. The AST preserves their position relative
to surrounding nodes. The formatter treats them like any other node in the Doc IR.

Types of comments in CMake:
- Line comment: `# ...` (to end of line)
- Bracket comment: `#[=[ ... ]=]` (multi-line)

## Config precedence (highest → lowest)

1. CLI flags (`--line-width`, etc.)
2. `.cmake-format.toml` in the directory of the file being formatted
3. `.cmake-format.toml` walking up to repo root (or git root)
4. `~/.cmake-format.toml` (user default)
5. Built-in defaults

The full config schema is documented in `docs/src/content/docs/config.md`.
Config sections: `[format]`, `[style]`, `[markup]`, `[per_command.<name>]`.
Goal: match or exceed every useful option from the original cmake-format tool.

## Coding conventions

- All public types derive `Debug`, `Clone`.
- Errors use `thiserror`. Never use `.unwrap()` in library code; use `?`.
- Pest rule names: `snake_case`. AST enum variants: `PascalCase`.
- New formatter rules go in `formatter/node.rs` alongside the relevant AST variant.
- Every new formatting behaviour needs a snapshot test.
