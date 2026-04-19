// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

#[path = "../compare/legacy_pest.rs"]
mod legacy_pest;

use cmakefmt::error::ParseDiagnostic;
use cmakefmt::parser::ast::File;
use libfuzzer_sys::fuzz_target;

fn parse_new(source: &str) -> std::result::Result<File, ParseDiagnostic> {
    match cmakefmt::parser::parse(source) {
        Ok(file) => Ok(file),
        Err(cmakefmt::Error::Parse(err)) => Err(err.diagnostic),
        Err(other) => panic!("new parser hit unexpected error variant: {other:?}"),
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    match (legacy_pest::parse_reference(source), parse_new(source)) {
        (Ok(left), Ok(right)) => assert_eq!(left, right, "AST divergence"),
        (Err(left), Err(right)) => {
            assert_eq!(
                (left.line, left.column),
                (right.line, right.column),
                "error location divergence"
            );
        }
        (Ok(_), Err(err)) => panic!("new parser rejected valid input: {err:?}"),
        (Err(err), Ok(_)) => panic!(
            "new parser accepted input rejected by frozen parser at {}:{}",
            err.line, err.column
        ),
    }
});
