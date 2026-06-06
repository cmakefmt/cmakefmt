// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use cmakefmt::semantic::semantic_equivalent;
use cmakefmt::{format_source, Config};
use libfuzzer_sys::fuzz_target;

/// Fuzz target: format arbitrary input and assert the formatter's core
/// guarantees on any input it accepts:
///
/// 1. **No panics** — reaching the asserts at all means formatting didn't crash.
/// 2. **Semantic preservation** — `format(x)` has the same commands and
///    arguments as `x` (ignoring comments, whitespace, and case), using the
///    same `--verify` checker the CLI uses. `semantic_equivalent` returns
///    `true` when either side fails to parse, so this never false-positives on
///    inputs the parser rejects.
/// 3. **Idempotency** — `format(format(x)) == format(x)`.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(formatted) = format_source(text, &Config::default()) else {
        return;
    };

    assert!(
        semantic_equivalent(text, &formatted),
        "formatting changed semantics:\n--- input ---\n{text}\n--- output ---\n{formatted}"
    );

    let reformatted = format_source(&formatted, &Config::default())
        .expect("re-formatting already-formatted output must succeed");
    assert_eq!(
        formatted, reformatted,
        "formatting is not idempotent:\n--- first ---\n{formatted}\n--- second ---\n{reformatted}"
    );
});
