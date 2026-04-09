// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz target for the CMake parser.
//!
//! Feeds arbitrary byte sequences to the parser to find panics, infinite
//! loops, or excessive memory allocation.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = cmakefmt::parser::parse(input);
    }
});
