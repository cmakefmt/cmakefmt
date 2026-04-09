// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Comment formatting helpers.

use crate::config::{CompiledPatterns, Config};
use crate::parser::ast::Comment;

/// Format a single comment node into one or more rendered output lines.
///
/// Bracket comments are preserved verbatim. Line comments may be reflowed when
/// markup handling and comment reflow are enabled in [`Config`].
pub fn format_comment_lines(
    comment: &Comment,
    config: &Config,
    patterns: &CompiledPatterns,
    indent_width: usize,
    line_width: usize,
) -> Vec<String> {
    match comment {
        Comment::Bracket(raw) => raw
            .replace("\r\n", "\n")
            .split('\n')
            .map(str::to_owned)
            .collect(),
        Comment::Line(text) => {
            format_line_comment(text, config, patterns, indent_width, line_width)
        }
    }
}

fn format_line_comment(
    text: &str,
    config: &Config,
    patterns: &CompiledPatterns,
    indent_width: usize,
    line_width: usize,
) -> Vec<String> {
    if !config.enable_markup
        || !config.reflow_comments
        || should_preserve_comment_verbatim(text, patterns)
    {
        return vec![text.to_owned()];
    }

    let body = text.trim_start_matches('#').trim_start();
    if body.is_empty() {
        return vec!["#".to_owned()];
    }

    let available = line_width.saturating_sub(indent_width);
    if available <= 3 || text.chars().count() <= available {
        return vec![text.to_owned()];
    }

    let prefix = "# ";
    let prefix_width = prefix.chars().count();
    if available <= prefix_width + 1 {
        return vec![text.to_owned()];
    }

    let mut lines = Vec::new();
    let mut current = String::from(prefix);
    let mut current_width = prefix_width;

    for word in body.split_whitespace() {
        let word_width = word.chars().count();
        let projected = if current_width == prefix_width {
            prefix_width + word_width
        } else {
            current_width + 1 + word_width
        };

        if projected > available && current_width != prefix_width {
            lines.push(current);
            current = String::with_capacity(prefix.len() + word.len());
            current.push_str(prefix);
            current.push_str(word);
            current_width = prefix_width + word_width;
        } else {
            if current_width != prefix_width {
                current.push(' ');
                current_width += 1;
            }
            current.push_str(word);
            current_width += word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn should_preserve_comment_verbatim(text: &str, patterns: &CompiledPatterns) -> bool {
    let trimmed = text.trim();

    if trimmed == "#" || trimmed.starts_with("#[[") || trimmed.starts_with("#[=[") {
        return true;
    }

    if trimmed.starts_with("# ~~~")
        || trimmed.contains("cmake-format:")
        || trimmed.contains("cmakefmt:")
    {
        return true;
    }

    if let Some(re) = &patterns.literal_comment {
        if re.is_match(text) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reflow_config() -> Config {
        Config {
            enable_markup: true,
            reflow_comments: true,
            ..Config::default()
        }
    }

    #[test]
    fn bracket_comments_preserve_newlines() {
        let config = Config::default();
        let patterns = config.compiled_patterns();
        let comment = Comment::Bracket("#[[a\r\nb]]".to_owned());
        let lines = format_comment_lines(&comment, &config, &patterns, 0, 80);
        assert_eq!(lines, vec!["#[[a".to_owned(), "b]]".to_owned()]);
    }

    #[test]
    fn line_comment_reflows_when_enabled() {
        let config = reflow_config();
        let patterns = config.compiled_patterns();
        let comment = Comment::Line(
            "# this is a long comment that should wrap when line width is small".to_owned(),
        );
        let lines = format_comment_lines(&comment, &config, &patterns, 2, 24);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.starts_with("# ")));
    }

    #[test]
    fn line_comment_preserves_for_barrier_directive() {
        let config = reflow_config();
        let patterns = config.compiled_patterns();
        let original = "# cmake-format: off".to_owned();
        let comment = Comment::Line(original.clone());
        let lines = format_comment_lines(&comment, &config, &patterns, 0, 8);
        assert_eq!(lines, vec![original]);
    }

    #[test]
    fn line_comment_preserves_when_matching_literal_pattern() {
        let mut config = reflow_config();
        config.literal_comment_pattern = r"^#\s*DO_NOT_WRAP".to_owned();
        let patterns = config.compiled_patterns();
        let original = "# DO_NOT_WRAP this must stay as-is".to_owned();
        let comment = Comment::Line(original.clone());
        let lines = format_comment_lines(&comment, &config, &patterns, 0, 8);
        assert_eq!(lines, vec![original]);
    }
}
