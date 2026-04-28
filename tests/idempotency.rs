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

#[test]
fn formatter_is_idempotent_and_preserves_parse_tree() {
    let config = Config::default();
    let registry = CommandRegistry::load().unwrap();

    for path in formatter_fixture_paths(Path::new("tests/fixtures")) {
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

        for (line_no, line) in first.lines().enumerate() {
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
