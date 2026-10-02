---
title: Upgrading to 2.0
description: Rust API migration, argument comment behaviour, and build requirements for cmakefmt 2.0.
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

Version 2.0 introduces breaking Rust API changes, preserves original argument
comment placement by default, and adds a dprint WebAssembly plugin. CLI flag
names, configuration-file syntax, and the LSP interface remain unchanged.

## Rust library consumers

Update the `cmakefmt-rust` dependency requirement to major version `2` when the
release is available. Source builds require Rust 1.88 or later.

The parser now distinguishes comments on their own lines
(`Argument::StandaloneComment`) from comments following an argument on the
same line (`Argument::InlineComment`). `Argument` is non-exhaustive: include
a fallback arm when matching it. Do not silently treat unknown variants as
comments or ordinary tokens when semantic correctness depends on the variant.

```rust
use cmakefmt::parser::ast::Argument;

fn describe(argument: &Argument) -> &'static str {
    match argument {
        Argument::StandaloneComment(_) => "standalone comment",
        Argument::InlineComment(_) => "inline comment",
        Argument::Bracket(_) | Argument::Quoted(_) | Argument::Unquoted(_) => "value",
        _ => "unsupported argument variant",
    }
}
```

Use `argument.is_comment()` if you only need to distinguish comments from
values, and `argument.as_str()` to obtain their raw text.

`Config` has a new `argument_comment_style` field. Rather than listing every
field, use defaults with explicit overrides:

```rust
use cmakefmt::{ArgumentCommentStyle, Config};

let config = Config {
    line_width: 100,
    argument_comment_style: ArgumentCommentStyle::Preserve,
    ..Config::default()
};
```

`Config` remains an exhaustive struct; this construction pattern avoids
listing all fields, but is not a promise that future field additions are
source-compatible with complete struct literals.

## Formatting and CI

The default `format.argument_comment_style: preserve` keeps standalone
argument comments on their own lines and inline comments attached where they
fit. Use `standalone` to put every argument comment on a separate line:

```yaml
format:
  argument_comment_style: standalone
```

Neither setting restores the old behaviour of moving standalone comments onto
argument lines. For example, start with this mixture of inline and standalone
comments:

```cmake
set(SOURCES
  main.cpp # entry point
  # platform implementation
  platform.cpp)
```

With `argument_comment_style: preserve` (the default), the inline comment stays
beside `main.cpp`, while the standalone comment stays above `platform.cpp`:

```cmake
set(SOURCES
    main.cpp # entry point
    # platform implementation
    platform.cpp)
```

With `argument_comment_style: standalone`, the inline comment moves onto a new
line **after** the argument it followed. The existing standalone comment stays
in place:

```cmake
set(SOURCES
    main.cpp
    # entry point
    # platform implementation
    platform.cpp)
```

These examples use the other default formatting settings. In `preserve` mode,
an inline comment can still move to a new line if it does not fit within
`line_width`; preserving placement does not override the line-width limit.

Review the output before updating formatting baselines:

```bash
cmakefmt --diff .
cmakefmt --check .
```

If the changes are appropriate, run `cmakefmt -i .`, review and commit the diff,
then rerun your formatting checks. Correctness fixes may also change affected
`FetchContent_Declare` and `add_subdirectory` calls.

Prebuilt binaries and Python wheels do not require Rust. Workflows pinning a
formatter version need an explicit update; workflows using `latest` may pick
up 2.0 automatically. The GitHub Action's own version is separate from the
formatter's version: keep the action version you use and set its `version`
input to `2.0.0` after publication if you want an explicit formatter pin.

## WebAssembly builds

Browser exports now require an explicit feature:

```bash
wasm-pack build --target web --no-default-features --features browser-wasm
```

The dprint plugin uses `dprint-plugin` instead. Do not combine the two features
when building a WebAssembly artifact; they expose different host protocols.
See [Editor Integration](/editors/) for dprint configuration.
