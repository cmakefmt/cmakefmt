// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz target for the CMake formatter.
//!
//! Parses arbitrary input and, if it parses successfully, formats it with
//! the default config. Catches panics or hangs in the formatting logic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = cmakefmt::format_source(input, &cmakefmt::Config::default());
    }
});
