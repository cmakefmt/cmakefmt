// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error rendering for the CLI: classify parse failures, format snippets, and
//! attach diagnostic hints. Kept here so `main.rs` can stay focused on
//! orchestration rather than diagnostic prose.

use std::fmt::Write as _;
use std::path::Path;

pub(crate) fn render_cli_error(err: &cmakefmt::Error) -> String {
    match err {
        cmakefmt::Error::Parse(parse) => render_parse_error(
            &parse.display_name,
            &parse.source_text,
            parse.start_line,
            &parse.diagnostic,
        ),
        cmakefmt::Error::Config(config) => {
            render_file_parse_error("config", &config.path, &config.details)
        }
        cmakefmt::Error::Spec(spec) => render_file_parse_error("spec", &spec.path, &spec.details),
        cmakefmt::Error::Formatter(message) => render_formatter_error(message),
        cmakefmt::Error::CliArg { message, .. } => format!("error: {message}"),
        cmakefmt::Error::InvalidRegex {
            pattern, source, ..
        } => format!(
            "error: invalid regex {pattern:?}: {source}\n\
             hint: Rust regex syntax does not support every PCRE feature; \
             see https://docs.rs/regex for the supported grammar"
        ),
        cmakefmt::Error::Render {
            format, message, ..
        } => format!("error: failed to render {format}: {message}"),
        cmakefmt::Error::LegacyMigration { path, message, .. } => format!(
            "error: legacy migration failed for {}: {message}",
            path.display()
        ),
        cmakefmt::Error::Io(source) => format!("error: I/O failure: {source}"),
        cmakefmt::Error::IoAt { path, source, .. } => {
            format!("error: I/O failure reading {}: {source}", path.display())
        }
        cmakefmt::Error::LayoutTooWide {
            line_no,
            width,
            limit,
            ..
        } => format!(
            "error: line {line_no} is {width} characters wide, exceeding the limit of {limit}\n\
             hint: set line_width = {width} (or higher), add the command to always_wrap, or disable require_valid_layout"
        ),
        _ => format!("error: {err}"),
    }
}

fn render_parse_error(
    display_name: &str,
    source_text: &str,
    start_line: usize,
    diagnostic: &cmakefmt::error::ParseDiagnostic,
) -> String {
    let local_line = diagnostic.line;
    let local_column = diagnostic.column;
    let absolute_line = start_line + local_line.saturating_sub(1);
    let source_lines: Vec<&str> = source_text.lines().collect();
    let line_text = source_lines
        .get(local_line.saturating_sub(1))
        .copied()
        .or_else(|| source_lines.last().copied())
        .unwrap_or_default();
    let (summary, mut hints) = classify_parse_failure(display_name, line_text, diagnostic);

    // If the error is at or near the end of the file, look for an unmatched
    // opening parenthesis. When found, show the error at the unclosed `(`
    // instead of at EOF — that's where the user needs to look.
    let is_near_eof = local_line >= source_lines.len().saturating_sub(1);
    let unmatched = if is_near_eof {
        find_unmatched_open_paren(source_text, start_line)
    } else {
        None
    };

    let mut rendered = String::new();
    if let Some((open_line, open_col, _)) = &unmatched {
        let _ = writeln!(
            rendered,
            "error: {summary}\n  --> {display_name}:{open_line}:{open_col}"
        );
        if !source_text.is_empty() {
            let open_local_line = open_line.saturating_sub(start_line) + 1;
            rendered.push('\n');
            rendered.push_str(&render_source_snippet(
                source_text,
                start_line,
                open_local_line,
                *open_col,
            ));
            rendered.push('\n');
        }
        hints.insert(0, "unclosed `(` — the closing `)` is missing".to_owned());
    } else {
        let _ = writeln!(
            rendered,
            "error: {summary}\n  --> {display_name}:{absolute_line}:{local_column}"
        );
        if !source_text.is_empty() {
            rendered.push('\n');
            rendered.push_str(&render_source_snippet(
                source_text,
                start_line,
                local_line,
                local_column,
            ));
            rendered.push('\n');
        }
    }
    for hint in hints {
        let _ = writeln!(rendered, "hint: {hint}");
    }
    let _ = writeln!(rendered, "parser detail: {}", diagnostic.message);
    if display_name != "<stdin>" {
        let _ = writeln!(rendered, "repro: cmakefmt --debug --check {display_name}");
    }
    rendered.trim_end().to_owned()
}

fn render_file_parse_error(
    kind: &str,
    path: &Path,
    source: &cmakefmt::error::FileParseError,
) -> String {
    let contents = std::fs::read_to_string(path).ok();
    let detail = source.message.as_ref();
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "error: invalid {kind} file ({})\n  --> {}",
        source.format,
        path.display()
    );

    if let (Some(contents), Some(line), Some(column)) =
        (contents.as_deref(), source.line, source.column)
    {
        let _ = writeln!(rendered, "      at {line}:{column}");
        rendered.push('\n');
        rendered.push_str(&render_source_snippet(contents, 1, line, column));
        rendered.push('\n');
    }

    let mut hints = Vec::new();
    if let Some((field, expected)) = extract_unknown_field_hint(detail) {
        if let Some(updated) = renamed_config_key(&field) {
            hints.push(format!(
                "`{field}` is not a valid cmakefmt key; use `{updated}` or run `cmakefmt config convert`"
            ));
        } else if let Some(suggestion) = best_match(&field, &expected) {
            hints.push(format!(
                "unknown key `{field}`; did you mean `{suggestion}`?"
            ));
        } else {
            hints.push(format!("unknown key `{field}` in {kind} file"));
        }
    }
    if kind == "config" {
        hints.push(
            "config files are applied in order; later files override earlier ones".to_owned(),
        );
    }
    for hint in hints {
        let _ = writeln!(rendered, "hint: {hint}");
    }
    let _ = writeln!(rendered, "detail: {detail}");
    rendered.trim_end().to_owned()
}

fn render_formatter_error(message: &str) -> String {
    format!("error: {message}")
}

/// Scan `source` for the last unmatched opening parenthesis, skipping
/// characters inside strings and comments. Returns `(line, column, context)`
/// where context is the trimmed source line containing the `(`.
fn find_unmatched_open_paren(source: &str, start_line: usize) -> Option<(usize, usize, String)> {
    // Stack of (line, column) for each unmatched '('.
    let mut paren_stack: Vec<(usize, usize)> = Vec::new();
    let mut line = 1usize;
    let mut col = 1usize;
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        match ch {
            '\n' => {
                line += 1;
                col = 1;
                i += 1;
            }
            '#' => {
                // Skip comment to end of line.
                i += 1;
                col += 1;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
            }
            '"' => {
                // Skip quoted string.
                i += 1;
                col += 1;
                while i < chars.len() {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                        col += 2;
                    } else if chars[i] == '"' {
                        i += 1;
                        col += 1;
                        break;
                    } else if chars[i] == '\n' {
                        line += 1;
                        col = 1;
                        i += 1;
                    } else {
                        i += 1;
                        col += 1;
                    }
                }
            }
            '(' => {
                paren_stack.push((line, col));
                i += 1;
                col += 1;
            }
            ')' => {
                paren_stack.pop();
                i += 1;
                col += 1;
            }
            _ => {
                i += 1;
                col += 1;
            }
        }
    }

    // The last unmatched '(' is the most likely culprit.
    let (open_line, open_col) = paren_stack.last().copied()?;
    let source_lines: Vec<&str> = source.lines().collect();
    let context = source_lines
        .get(open_line.saturating_sub(1))
        .map(|l| l.trim().to_owned())
        .unwrap_or_default();
    let absolute_line = start_line + open_line.saturating_sub(1);
    Some((absolute_line, open_col, context))
}

fn classify_parse_failure(
    display_name: &str,
    line_text: &str,
    diagnostic: &cmakefmt::error::ParseDiagnostic,
) -> (String, Vec<String>) {
    let detail = diagnostic.message.as_ref();
    let trimmed = line_text.trim();
    let mut hints = Vec::new();

    if line_text.contains("\\\"") {
        hints.push(
            "possible malformed quoted argument or escaped quote sequence inside this command"
                .to_owned(),
        );
        hints.push(
            "if the caret looks late, the real problem may be earlier in the same command invocation"
                .to_owned(),
        );
        return ("failed to parse a quoted argument".to_owned(), hints);
    }

    if trimmed.contains("[=[") || trimmed.contains("[[") || trimmed.contains("]=]") {
        hints.push(
            "check that bracket argument or bracket comment delimiters use matching `=` counts"
                .to_owned(),
        );
        return (
            "failed to parse a bracket argument or comment".to_owned(),
            hints,
        );
    }

    if display_name.ends_with(".cmake.in") && trimmed.starts_with('@') && trimmed.ends_with('@') {
        hints.push("top-level configure-file placeholders like @PACKAGE_INIT@ are only valid as standalone template lines".to_owned());
        return (
            "failed to parse a configure-file template line".to_owned(),
            hints,
        );
    }

    if trimmed.contains('(') || trimmed.contains(')') {
        hints.push(
            "check for an unbalanced command invocation or control-flow condition".to_owned(),
        );
        hints.push(
            "the reported location can be after the real problem if an earlier line left the parser out of sync"
                .to_owned(),
        );
        return ("failed to parse a command invocation".to_owned(), hints);
    }

    if detail.contains("quoted element") {
        hints.push("a quoted string may be unterminated or contain malformed escapes".to_owned());
    }

    ("failed to parse CMake input".to_owned(), hints)
}

fn render_source_snippet(
    source: &str,
    start_line: usize,
    focus_line: usize,
    focus_column: usize,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let focus_index = focus_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let start_index = focus_index.saturating_sub(1);
    let end_index = (focus_index + 2).min(lines.len());
    let max_line_no = start_line + end_index.saturating_sub(1);
    let width = max_line_no.to_string().len();
    let mut rendered = String::new();

    for index in start_index..end_index {
        let absolute_line = start_line + index;
        let marker = if index == focus_index { '>' } else { ' ' };
        let _ = writeln!(
            rendered,
            "{marker} {absolute_line:>width$} | {}",
            lines[index],
            width = width
        );
        if index == focus_index {
            let visible_column = if focus_line > lines.len() {
                lines[index].chars().count() + 1
            } else {
                focus_column
            };
            let caret_padding = visible_column.saturating_sub(1);
            let _ = writeln!(
                rendered,
                "  {space:>width$} | {pad}^",
                space = "",
                pad = " ".repeat(caret_padding),
                width = width
            );
        }
    }

    rendered.trim_end().to_owned()
}

fn extract_unknown_field_hint(detail: &str) -> Option<(String, Vec<String>)> {
    let field = extract_between(detail, "unknown field `", "`")
        .or_else(|| extract_between(detail, "unknown field '", "'"))?;
    let expected = detail
        .split("expected one of")
        .nth(1)
        .map(|tail| {
            tail.split(',')
                .map(|part| {
                    part.trim()
                        .trim_matches('`')
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_owned()
                })
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some((field, expected))
}

fn extract_between(input: &str, start: &str, end: &str) -> Option<String> {
    let tail = input.split(start).nth(1)?;
    let field = tail.split(end).next()?;
    Some(field.to_owned())
}

fn best_match<'a>(needle: &str, candidates: &'a [String]) -> Option<&'a str> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let distance = levenshtein(needle, candidate);
            (distance <= 6).then_some((distance, candidate.as_str()))
        })
        .min_by_key(|(distance, candidate)| (*distance, candidate.len()))
        .map(|(_, candidate)| candidate)
}

fn levenshtein(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let mut prev: Vec<usize> = (0..=right.len()).collect();
    let mut curr = vec![0; right.len() + 1];

    for (i, lch) in left.iter().enumerate() {
        curr[0] = i + 1;
        for (j, rch) in right.iter().enumerate() {
            let cost = usize::from(lch != rch);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[right.len()]
}

fn renamed_config_key(key: &str) -> Option<&'static str> {
    match key {
        "use_tabchars" => Some("use_tabs"),
        "max_lines_hwrap" => Some("max_hanging_wrap_lines"),
        "max_pargs_hwrap" => Some("max_hanging_wrap_positional_args"),
        "max_subgroups_hwrap" => Some("max_hanging_wrap_groups"),
        "min_prefix_chars" => Some("min_prefix_length"),
        "max_prefix_chars" => Some("max_prefix_length"),
        "separate_ctrl_name_with_space" => Some("space_before_control_paren"),
        "separate_fn_name_with_space" => Some("space_before_definition_paren"),
        _ => None,
    }
}
