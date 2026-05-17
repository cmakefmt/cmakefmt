// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz target: feed arbitrary bytes into `cmakefmt::parser::parse`.
//!
//! The contract this harness asserts is narrow but important: the parser
//! must never panic, abort, or hang on any input — regardless of whether
//! the input is well-formed CMake, malformed CMake, or random noise.
//! Returning `Err(_)` for malformed input is fine and expected; crashing
//! is not.
//!
//! Bytes are decoded with `String::from_utf8_lossy` rather than
//! `from_utf8` so the fuzzer can explore invalid-UTF-8 inputs without
//! short-circuiting on the decoder. Replacement characters round-trip
//! through the parser the same way any other Unicode codepoint would.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    let _ = cmakefmt::parser::parse(&source);
});
