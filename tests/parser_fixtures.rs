// SPDX-FileCopyrightText: 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::Path;

use cmakefmt::parser::parse;
use walkdir::WalkDir;

fn fixture_paths(root: &Path) -> Vec<std::path::PathBuf> {
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

#[test]
fn all_parser_fixtures_parse() {
    let root = Path::new("tests/fixtures");

    for path in fixture_paths(root) {
        let source = fs::read_to_string(&path).unwrap();
        parse(&source).unwrap_or_else(|err| panic!("fixture {} failed: {err}", path.display()));
    }
}
