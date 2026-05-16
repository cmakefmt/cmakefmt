// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

use cmakefmt::{
    format_source, parser,
    parser::ast::{File, Statement},
    semantic::{normalize_command_literals, normalize_keyword_args, normalize_line_endings},
    spec::registry::CommandRegistry,
    Config,
};
use walkdir::WalkDir;

fn formatter_fixture_paths(root: &Path) -> Vec<PathBuf> {
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

/// Locate the real-world corpus directory if it has been fetched.
///
/// Returns `None` if the corpus is absent (no `target/real-world-corpus`
/// and `CMAKEFMT_REAL_WORLD_DIR` unset), so the idempotency test
/// degrades to the in-tree fixture corpus rather than failing. CI fetches
/// the corpus via `scripts/fetch-real-world-corpus.py` before invoking
/// `cargo test`, so this path is exercised on every PR; local runs that
/// haven't fetched the corpus still pass on the smaller in-tree set.
fn real_world_corpus_root() -> Option<PathBuf> {
    let root = std::env::var_os("CMAKEFMT_REAL_WORLD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/real-world-corpus"));
    if root.exists() {
        Some(root)
    } else {
        None
    }
}

/// Core safety-net properties checked on every available input:
/// idempotency (`format(format(x)) == format(x)`) and semantic
/// preservation (the formatter must not change the parse tree
/// modulo whitespace and comments).
///
/// Runs against the curated in-tree fixtures plus the real-world
/// corpus when present (see `real_world_corpus_root`). The
/// line-width assertion is split out below because real-world
/// CMakeLists.txt files routinely contain legitimately-unbreakable
/// constructs that the heuristic line-width guard can't model;
/// keeping them out of the line-width check lets us still exercise
/// idempotency and semantic preservation against ~20 upstream
/// projects on every PR.
#[test]
fn formatter_is_idempotent_and_preserves_parse_tree() {
    let config = Config::default();
    let registry = CommandRegistry::load().unwrap();

    let mut paths = formatter_fixture_paths(Path::new("tests/fixtures"));
    if let Some(root) = real_world_corpus_root() {
        paths.extend(formatter_fixture_paths(&root));
    }

    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let first = format_source(&source, &config)
            .unwrap_or_else(|err| panic!("first format failed for {}: {err}", path.display()));
        let second = format_source(&first, &config)
            .unwrap_or_else(|err| panic!("second format failed for {}: {err}", path.display()));

        assert_eq!(
            first,
            second,
            "formatter was not idempotent for {}",
            path.display()
        );

        let original_ast = parser::parse(&source).unwrap();
        let formatted_ast = parser::parse(&first).unwrap();
        assert_eq!(
            normalize_semantics(original_ast, &registry),
            normalize_semantics(formatted_ast, &registry),
            "formatted output changed parse tree for {}",
            path.display()
        );
    }
}

/// Line-width compliance check, restricted to the curated in-tree
/// fixtures. The real-world corpus is excluded because upstream
/// CMakeLists.txt files contain unbreakable constructs (variable
/// references with long names inside genex, deeply nested
/// `if(...AND...OR...)` chains, etc.) that the formatter cannot
/// shorten without rewriting source the user wrote by hand. Lines
/// like that are a known gap in the line-width contract, tracked on
/// the roadmap rather than enforced here against arbitrary upstream
/// projects.
#[test]
fn formatter_respects_line_width_on_curated_fixtures() {
    let config = Config::default();

    for path in formatter_fixture_paths(Path::new("tests/fixtures")) {
        let source = fs::read_to_string(&path).unwrap();
        let formatted = format_source(&source, &config)
            .unwrap_or_else(|err| panic!("format failed for {}: {err}", path.display()));

        for (line_no, line) in formatted.lines().enumerate() {
            if line_contains_comment(line) || line_has_unbreakable_literal(line) {
                continue;
            }
            assert!(
                line.chars().count() <= config.line_width,
                "{}:{} exceeded line width {} with {} chars",
                path.display(),
                line_no + 1,
                config.line_width,
                line.chars().count()
            );
        }
    }
}

fn normalize_semantics(mut file: File, registry: &CommandRegistry) -> File {
    // Strip standalone comments and blank lines — they have no CMake semantic
    // meaning and may change structure when the formatter reflows them.
    // Keep in sync with src/main.rs::normalize_semantics.
    file.statements
        .retain(|s| !matches!(s, Statement::Comment(_) | Statement::BlankLines(_)));

    for statement in &mut file.statements {
        match statement {
            Statement::Command(command) => {
                command.span = (0, 0);
                command.name.make_ascii_lowercase();
                normalize_command_literals(command);
                normalize_keyword_args(command, registry);
            }
            Statement::TemplatePlaceholder(value) => normalize_line_endings(value),
            Statement::Comment(_) | Statement::BlankLines(_) => unreachable!(),
        }
    }

    file
}

fn line_contains_comment(line: &str) -> bool {
    line.contains('#')
}

fn line_has_unbreakable_literal(line: &str) -> bool {
    line.contains('"')
        || line.contains("[[")
        || line.contains("[=")
        || line.contains("$<")
        || line.trim_start().starts_with('(')
}
