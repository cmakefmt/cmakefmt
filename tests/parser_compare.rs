// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[path = "compare/legacy_pest.rs"]
mod legacy_pest;

use std::fs;
use std::path::{Path, PathBuf};

use cmakefmt::error::ParseDiagnostic;
use cmakefmt::parser::ast::File;
use walkdir::WalkDir;

fn parse_new(source: &str) -> std::result::Result<File, ParseDiagnostic> {
    match cmakefmt::parser::parse(source) {
        Ok(file) => Ok(file),
        Err(cmakefmt::Error::Parse(err)) => Err(err.diagnostic),
        Err(other) => panic!("new parser hit unexpected error variant: {other:?}"),
    }
}

fn assert_equivalent(source: &str, label: &str) {
    match (legacy_pest::parse_reference(source), parse_new(source)) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "AST divergence for {label}"),
        (Err(a), Err(b)) => assert_eq!(
            (a.line, a.column),
            (b.line, b.column),
            "error location divergence for {label}"
        ),
        (Ok(_), Err(err)) => panic!("new parser rejects {label}: {err:?}"),
        (Err(err), Ok(_)) => panic!("new parser accepts invalid {label}: {err:?}"),
    }
}

/// Inputs where the legacy pest grammar is incorrect and the new parser is
/// right. These are kept visible (not silently dropped) so the list doubles
/// as documentation of known pre-existing bugs the rewrite resolves.
///
/// Entries are matched as path suffixes so they work regardless of the
/// walk root.
const LEGACY_PARSER_BUGS: &[&str] = &[
    // `[==[ ... [====[...]====] ... ]==]` — the pest rule
    // `bracket_close_any = "]" ~ "="* ~ "]"` stops bracket-content scanning
    // at any `]=*]`, including the inner `]====]` whose `=` count does not
    // match the opener. The new parser requires an exact `=` match.
    "benches/repos/cmake/Tests/RunCMake/cmake-E-bin2c/record_very_long.cmake",
];

fn is_known_legacy_bug(path: &Path) -> bool {
    let display = path.to_string_lossy();
    LEGACY_PARSER_BUGS
        .iter()
        .any(|suffix| display.ends_with(suffix))
}

fn fixture_paths(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cmake") || ext == "txt")
        })
        .map(|entry| entry.into_path())
        .collect()
}

fn repo_cmake_paths(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("CMakeLists.txt"))
                || entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("cmake") || ext == "txt")
        })
        .map(|entry| entry.into_path())
        .collect()
}

#[test]
fn compare_smoke_inputs() {
    for (label, input) in [
        ("simple-command", "cmake_minimum_required(VERSION 3.20)\n"),
        ("trailing-comment", "set(FOO bar) # trailing\n"),
        (
            "comment-continuation",
            "set(FOO bar) # first\n             # second\n",
        ),
        ("bracket-arg", "set(VAR [==[hello]==])\n"),
        ("genex", "target_link_libraries(foo $<TARGET_FILE:bar>)\n"),
        ("mixed-unquoted", "set(x -Da=\"b c\")\n"),
        (
            "quoted-prefix-unquoted",
            r##"list(APPEND force-libcxx "CMAKE_CXX_COMPILER_ID STREQUAL "Clang"")"##,
        ),
        ("template-placeholder", "@PACKAGE_INIT@\n"),
        ("bom", "\u{FEFF}project(MyProject)\n"),
    ] {
        assert_equivalent(input, label);
    }
}

#[test]
fn compare_hand_curated_fixtures() {
    for path in fixture_paths(Path::new("tests/fixtures")) {
        let source = fs::read_to_string(&path).unwrap();
        assert_equivalent(&source, &path.display().to_string());
    }
}

#[test]
#[ignore = "heavy corpus comparison"]
fn compare_real_world_corpus() {
    let root = std::env::var_os("CMAKEFMT_REAL_WORLD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/real-world-corpus"));
    if !root.exists() {
        eprintln!(
            "skipping real-world compare corpus: {} does not exist",
            root.display()
        );
        return;
    }

    for path in fixture_paths(&root) {
        if is_known_legacy_bug(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        assert_equivalent(&source, &path.display().to_string());
    }
}

#[test]
#[ignore = "heavy repository comparison"]
fn compare_bench_repositories() {
    let root = Path::new("benches/repos");
    if !root.exists() {
        return;
    }

    let mut failures = Vec::new();
    for path in repo_cmake_paths(root) {
        if is_known_legacy_bug(&path) {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue; // non-UTF8 files are outside the harness scope
        };
        let label = path.display().to_string();
        match (legacy_pest::parse_reference(&source), parse_new(&source)) {
            (Ok(a), Ok(b)) if a == b => {}
            (Err(a), Err(b)) if (a.line, a.column) == (b.line, b.column) => {}
            (lhs, rhs) => failures.push(format!("{label}: legacy={lhs:?} new={rhs:?}")),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} bench-repo divergences:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
