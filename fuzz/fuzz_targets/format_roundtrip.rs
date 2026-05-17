// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz target: parse → format → parse round-trip.
//!
//! We assert only the *absence of crashes*:
//!
//! * If the first parse succeeds, formatting must not panic.
//! * If formatting succeeds, parsing the formatted output must not
//!   panic.
//!
//! AST-equivalence between the input and the re-parsed formatted output
//! is deliberately NOT asserted here — that's the verifier's job and is
//! covered by the integration test suite. Coverage-guided fuzzing is
//! aimed at finding panics, infinite loops, and OOMs that the existing
//! tests miss.
//!
//! As with `parse.rs`, raw bytes are decoded via
//! `String::from_utf8_lossy` so the fuzzer can explore invalid-UTF-8
//! inputs.

#![no_main]

use cmakefmt::{format_source, Config};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);

    // First parse. If it fails we've still exercised the parser; no
    // round-trip is possible.
    if cmakefmt::parser::parse(&source).is_err() {
        return;
    }

    // Formatting must not panic on input the parser accepted. Returning
    // `Err(_)` is acceptable (e.g. semantic-layer rejections); panics
    // are not.
    let formatted = match format_source(&source, &Config::default()) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Re-parsing the formatter's own output must also not panic.
    let _ = cmakefmt::parser::parse(&formatted);
});
