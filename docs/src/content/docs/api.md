---
title: Library API
description: Embed cmakefmt in Rust code to parse, format, and inspect CMake files in-process — no subprocess required.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

`cmakefmt` is primarily a CLI tool, but the crate also exposes a capable
embedded API for Rust code that needs to parse or format CMake sources
in-process — no subprocess, no shell escaping, no overhead.

## When To Use The Library

The crate is a strong fit when you want to:

- format generated CMake from Rust code
- build editor or IDE tooling around CMake formatting
- run `cmakefmt` in-process instead of spawning a subprocess
- experiment with custom command registries
- parse CMake and inspect the AST directly

## Crate Status

The Rust library API is intended to be stable for the `1.x` line.

The stable contract is:

- top-level formatting entry points such as `format_source` and `format_parsed_file`
- configuration loading and merging through `Config`
- the public AST returned by `cmakefmt::parser::parse`
- command-registry loading and override APIs through `CommandRegistry`
- crate-owned error diagnostics through `cmakefmt::Error`

Internal implementation details are not part of that contract. That includes
the parser engine, grammar-rule names, and other private helper types.

## Public Entry Points

The most important items today:

- `format_source`
- `format_source_with_debug`
- `format_source_with_registry`
- `format_source_with_registry_debug`
- `Config`
- `CaseStyle`
- `DangleAlign`
- `PerCommandConfig`
- `Error`
- `Result`

Lower-level access is available through:

- `cmakefmt::parser`
- `cmakefmt::spec::registry::CommandRegistry`

## Minimal Formatting Example

```rust
use cmakefmt::{Config, format_source};

fn main() -> Result<(), cmakefmt::Error> {
    let src = "target_link_libraries(foo PUBLIC bar)";
    let out = format_source(src, &Config::default())?;
    println!("{out}");
    Ok(())
}
```

The simplest entry point when you already have source text in memory.

## Formatting With A Tweaked Config

```rust
use cmakefmt::{CaseStyle, Config, format_source};

fn main() -> Result<(), cmakefmt::Error> {
    let mut config = Config::default();
    config.line_width = 100;
    config.command_case = CaseStyle::Lower;
    config.keyword_case = CaseStyle::Upper;
    config.dangle_parens = true;

    let src = r#"
add_library(foo STATIC a.cc b.cc)
target_link_libraries(foo PUBLIC bar baz)
"#;

    let out = format_source(src, &config)?;
    println!("{out}");
    Ok(())
}
```

Use this pattern when the application needs to supply formatter policy at
runtime rather than discovering it from disk.

## Loading Config From Disk

To use the same config-loading behavior the CLI uses:

```rust
use std::path::Path;

use cmakefmt::Config;

fn main() -> Result<(), cmakefmt::Error> {
    let config = Config::from_file(Path::new(".cmakefmt.yaml"))?;
    println!("line width: {}", config.line_width);
    Ok(())
}
```

Merge multiple explicit config files in precedence order:

```rust
use std::path::PathBuf;

use cmakefmt::Config;

fn main() -> Result<(), cmakefmt::Error> {
    let config = Config::from_files(&[
        PathBuf::from("base.yaml"),
        PathBuf::from("team.yaml"),
    ])?;
    println!("{:#?}", config);
    Ok(())
}
```

Ask which config files would be discovered for a given target:

```rust
use std::path::Path;

use cmakefmt::Config;

fn main() {
    let sources = Config::config_sources_for(Path::new("src/CMakeLists.txt"));
    for path in sources {
        println!("{}", path.display());
    }
}
```

## Formatting With Debug Decisions

Building tooling and want insight into what the formatter decided? Use the
debug variant:

```rust
use cmakefmt::{Config, format_source_with_debug};

fn main() -> Result<(), cmakefmt::Error> {
    let src = "install(TARGETS mylib DESTINATION lib)";
    let (formatted, debug_lines) = format_source_with_debug(src, &Config::default())?;

    println!("{formatted}");
    for line in debug_lines {
        eprintln!("{line}");
    }

    Ok(())
}
```

The returned debug lines are the same formatter-decision detail that the CLI
emits under `--debug`.

## Using A Custom Command Registry

For syntax that is not part of the built-in registry, use `CommandRegistry`
directly:

```rust
use cmakefmt::{Config, format_source_with_registry};
use cmakefmt::spec::registry::CommandRegistry;

fn main() -> Result<(), cmakefmt::Error> {
    let mut registry = CommandRegistry::load()?;
    registry.merge_override_str(
        r#"
[commands.my_custom_command]
pargs = 1
flags = ["QUIET"]

[commands.my_custom_command.kwargs.SOURCES]
nargs = "+"
"#,
        "inline-override.toml",
    )?;

    let src = "my_custom_command(foo QUIET SOURCES a.cc b.cc)";
    let out = format_source_with_registry(src, &Config::default(), &registry)?;
    println!("{out}");
    Ok(())
}
```

This is the primary embedded path for generated or custom CMake DSLs.

## Parsing Without Formatting

When you only need the AST:

```rust
use cmakefmt::parser::parse;

fn main() -> Result<(), cmakefmt::Error> {
    let file = parse("project(example LANGUAGES CXX)")?;
    println!("{:#?}", file);
    Ok(())
}
```

Useful for analysis tools, migration tooling, or experiments that want the
CMake parse tree but not the formatter.

## Error Model

The library uses a shared `cmakefmt::Error` type across parsing, config
loading, registry loading, and formatting:

| Error kind | Meaning |
| --- | --- |
| `Error::Parse` | the input was not valid CMake under the current grammar, with crate-owned source and location diagnostics |
| `Error::Config` | a user config file failed to parse or validate |
| `Error::Spec` | a command-spec override or built-in spec failed to parse |
| `Error::Io` | file I/O failed |
| `Error::Formatter` | a formatter-layer invariant or unsupported case was hit |
| `Error::LayoutTooWide` | the debug layout renderer exceeded the configured line width |

For parse, config, and spec errors, the library retains crate-owned path and
location context so callers can surface useful diagnostics without depending on
parser-library or YAML-library internals.

## Current Limits

- the public API is smaller than the CLI feature surface
- workflow features like Git-aware selection and ignore-file handling live in the CLI layer, not the formatting API itself

For deeper implementation details, continue with [Architecture](/architecture/).
