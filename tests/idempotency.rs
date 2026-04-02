use std::fs;
use std::path::{Path, PathBuf};

use cmfmt::{
    format_source, parser,
    parser::ast::{Argument, File, Statement},
    spec::{registry::CommandRegistry, CommandForm, KwargSpec},
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
    for statement in &mut file.statements {
        if let Statement::Command(command) = statement {
            command.span = (0, 0);
            command.name.make_ascii_lowercase();
            normalize_keyword_args(command, registry);
        }
    }

    file
}

fn line_contains_comment(line: &str) -> bool {
    line.contains('#')
}

fn line_has_unbreakable_literal(line: &str) -> bool {
    line.contains('"') || line.contains("[[") || line.contains("[=") || line.contains("$<")
}

fn normalize_keyword_args(
    command: &mut cmfmt::parser::ast::CommandInvocation,
    registry: &CommandRegistry,
) {
    let spec = registry.get(&command.name);
    let first_arg = command.arguments.iter().find_map(first_arg_text);
    let form = spec.form_for(first_arg);
    let keyword_set = collect_keywords(form);

    for arg in &mut command.arguments {
        if let Argument::Unquoted(value) = arg {
            let upper = value.to_ascii_uppercase();
            if keyword_set.contains(upper.as_str()) {
                *value = upper;
            }
        }
    }
}

fn first_arg_text(argument: &Argument) -> Option<&str> {
    match argument {
        Argument::Quoted(_) | Argument::Bracket(_) | Argument::InlineComment(_) => None,
        Argument::Unquoted(value) => Some(value.as_str()),
    }
}

fn collect_keywords(form: &CommandForm) -> std::collections::BTreeSet<String> {
    let mut keywords = std::collections::BTreeSet::new();
    collect_form_keywords(form, &mut keywords);
    keywords
}

fn collect_form_keywords(form: &CommandForm, keywords: &mut std::collections::BTreeSet<String>) {
    keywords.extend(form.flags.iter().cloned());

    for (name, spec) in &form.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(spec, keywords);
    }
}

fn collect_kwarg_keywords(spec: &KwargSpec, keywords: &mut std::collections::BTreeSet<String>) {
    keywords.extend(spec.flags.iter().cloned());

    for (name, child) in &spec.kwargs {
        keywords.insert(name.clone());
        collect_kwarg_keywords(child, keywords);
    }
}
