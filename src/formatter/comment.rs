//! Comment formatting helpers.

use regex::Regex;

use crate::config::Config;
use crate::parser::ast::Comment;

/// Format a single comment node into one or more rendered output lines.
///
/// Bracket comments are preserved verbatim. Line comments may be reflowed when
/// markup handling and comment reflow are enabled in [`Config`].
pub fn format_comment_lines(
    comment: &Comment,
    config: &Config,
    indent_width: usize,
    line_width: usize,
) -> Vec<String> {
    match comment {
        Comment::Bracket(raw) => raw
            .replace("\r\n", "\n")
            .split('\n')
            .map(str::to_owned)
            .collect(),
        Comment::Line(text) => format_line_comment(text, config, indent_width, line_width),
    }
}

fn format_line_comment(
    text: &str,
    config: &Config,
    indent_width: usize,
    line_width: usize,
) -> Vec<String> {
    if !config.enable_markup
        || !config.reflow_comments
        || should_preserve_comment_verbatim(text, config)
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

    for word in body.split_whitespace() {
        let projected = if current == prefix {
            prefix_width + word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };

        if projected > available && current != prefix {
            lines.push(current);
            current = format!("{prefix}{word}");
        } else {
            if current != prefix {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn should_preserve_comment_verbatim(text: &str, config: &Config) -> bool {
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

    if !config.literal_comment_pattern.is_empty()
        && Regex::new(&config.literal_comment_pattern)
            .ok()
            .is_some_and(|pattern| pattern.is_match(text))
    {
        return true;
    }

    false
}
