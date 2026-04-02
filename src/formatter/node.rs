use crate::config::{CaseStyle, CommandConfig, Config, DangleAlign};
use crate::error::Result;
use crate::formatter::comment;
use crate::parser::ast::{Argument, CommandInvocation};
use crate::spec::registry::CommandRegistry;
use crate::spec::CommandForm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderKind {
    Keyword,
    Flag,
}

#[derive(Debug)]
struct Section<'a> {
    header: Option<&'a str>,
    header_kind: Option<HeaderKind>,
    arguments: Vec<&'a Argument>,
}

pub fn format_command(
    command: &CommandInvocation,
    config: &Config,
    registry: &CommandRegistry,
    block_depth: usize,
) -> Result<String> {
    let cmd_config = config.for_command(&command.name);
    let form = registry
        .get(&command.name)
        .form_for(first_argument(command).map(Argument::as_str));
    let sections = split_sections(command, form)?;

    let output = if let Some(inline) = try_format_inline(
        command,
        &sections,
        &cmd_config,
        block_depth,
        config.line_width,
    ) {
        inline
    } else if let Some(hanging) = try_format_hanging(
        command,
        &sections,
        &cmd_config,
        block_depth,
        config.line_width,
    ) {
        hanging
    } else {
        format_command_vertical(command, &sections, &cmd_config, block_depth)?
    };

    if config.use_tabchars {
        Ok(spaces_to_tabs(&output, cmd_config.tab_size()))
    } else {
        Ok(output)
    }
}

fn first_argument(command: &CommandInvocation) -> Option<&Argument> {
    command
        .arguments
        .iter()
        .find(|argument| !argument.is_comment())
}

fn format_name(command: &CommandInvocation, cmd_config: &CommandConfig<'_>) -> String {
    let name = apply_case(cmd_config.command_case(), &command.name);
    if cmd_config.space_before_paren {
        format!("{name} ")
    } else {
        name
    }
}

fn split_sections<'a>(
    command: &'a CommandInvocation,
    form: &'a CommandForm,
) -> Result<Vec<Section<'a>>> {
    let mut sections = Vec::new();

    for argument in &command.arguments {
        if argument.is_comment() {
            if sections.is_empty() {
                sections.push(Section {
                    header: None,
                    header_kind: None,
                    arguments: Vec::new(),
                });
            }
            sections
                .last_mut()
                .expect("section list contains at least one section")
                .arguments
                .push(argument);
            continue;
        }

        let token = argument.as_str();
        let normalized = token.to_ascii_uppercase();
        let header_kind = if form.kwargs.contains_key(&normalized) {
            Some(HeaderKind::Keyword)
        } else if form.flags.contains(&normalized) {
            Some(HeaderKind::Flag)
        } else {
            None
        };

        if let Some(header_kind) = header_kind {
            sections.push(Section {
                header: Some(token),
                header_kind: Some(header_kind),
                arguments: Vec::new(),
            });
            continue;
        }

        if sections.is_empty() {
            sections.push(Section {
                header: None,
                header_kind: None,
                arguments: Vec::new(),
            });
        }

        sections
            .last_mut()
            .expect("section list contains at least one section")
            .arguments
            .push(argument);
    }

    Ok(sections)
}

fn try_format_inline(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
    line_width: usize,
) -> Option<String> {
    if command.arguments.iter().any(argument_has_newline)
        || command.arguments.iter().any(Argument::is_comment)
    {
        return None;
    }

    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let mut output = format!("{base_indent}{}(", format_name(command, cmd_config));

    let mut first_token = true;
    for section in sections {
        if let Some(header) = section.header {
            if !first_token {
                output.push(' ');
            }
            output.push_str(&apply_case(cmd_config.keyword_case(), header));
            first_token = false;
        }

        for argument in &section.arguments {
            if !first_token {
                output.push(' ');
            }
            output.push_str(argument.as_str());
            first_token = false;
        }
    }

    output.push(')');
    (output.chars().count() <= line_width).then_some(output)
}

fn try_format_hanging(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
    line_width: usize,
) -> Option<String> {
    if command.arguments.iter().any(Argument::is_comment)
        || command.arguments.iter().any(argument_has_newline)
    {
        return None;
    }

    if sections.len() != 1 || sections[0].header.is_some() {
        return None;
    }

    let is_condition_command = matches!(
        command.name.to_ascii_lowercase().as_str(),
        "if" | "elseif" | "while"
    );

    if !is_condition_command && sections[0].arguments.len() > cmd_config.max_pargs_hwrap() {
        return None;
    }

    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let prefix = format!("{base_indent}{}(", format_name(command, cmd_config));
    let continuation = " ".repeat(prefix.chars().count());
    let tokens: Vec<String> = sections[0]
        .arguments
        .iter()
        .map(|argument| argument.as_str().to_owned())
        .collect();
    let break_before = match command.name.to_ascii_lowercase().as_str() {
        "if" | "elseif" | "while" => &["AND", "OR"][..],
        _ => &[][..],
    };

    let mut lines = pack_tokens(
        &prefix,
        &continuation,
        &tokens,
        line_width,
        cmd_config.global.max_lines_hwrap,
        break_before,
    )?;
    if lines.len() == 1 {
        lines[0].push(')');
        return Some(lines.remove(0));
    }

    Some(close_multiline(
        lines,
        &base_indent,
        format_name(command, cmd_config).len(),
        cmd_config,
    ))
}

fn format_command_vertical(
    command: &CommandInvocation,
    sections: &[Section<'_>],
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
) -> Result<String> {
    let base_indent = cmd_config.indent_str().repeat(block_depth);
    let indent = format!("{base_indent}{}", cmd_config.indent_str());
    let nested_indent = format!("{indent}{}", cmd_config.indent_str());
    let mut output = String::new();

    let name = format_name(command, cmd_config);
    output.push_str(&base_indent);
    output.push_str(&name);
    output.push_str("(\n");

    for section in sections {
        match section.header {
            None => {
                write_packed_arguments(
                    &mut output,
                    &section.arguments,
                    &indent,
                    cmd_config.global,
                    cmd_config.line_width(),
                );
            }
            Some(header) => {
                let header = apply_case(cmd_config.keyword_case(), header);
                if section.arguments.is_empty() {
                    output.push_str(&indent);
                    output.push_str(&header);
                    output.push('\n');
                    continue;
                }

                if let Some(line) = format_section_inline(
                    &header,
                    section.header_kind,
                    &section.arguments,
                    &indent,
                    cmd_config.global,
                    cmd_config.line_width(),
                ) {
                    output.push_str(&indent);
                    output.push_str(&line);
                    output.push('\n');
                    continue;
                }

                output.push_str(&indent);
                output.push_str(&header);
                output.push('\n');
                if section.arguments.len() > cmd_config.max_pargs_hwrap() {
                    write_vertical_arguments(
                        &mut output,
                        &section.arguments,
                        &nested_indent,
                        cmd_config.global,
                    );
                } else {
                    write_packed_arguments(
                        &mut output,
                        &section.arguments,
                        &nested_indent,
                        cmd_config.global,
                        cmd_config.line_width(),
                    );
                }
            }
        }
    }

    if output.ends_with('\n') {
        output.pop();
    }

    if cmd_config.dangle_parens() {
        output.push('\n');
        match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => output.push_str(&base_indent),
            DangleAlign::Open => {
                output.push_str(&base_indent);
                output.push_str(&" ".repeat(name.len()));
            }
        }
        output.push(')');
    } else if last_output_line_has_comment(&output) {
        output.push('\n');
        output.push_str(&base_indent);
        output.push(')');
    } else {
        output.push(')');
    }

    Ok(output)
}

fn format_section_inline(
    header: &str,
    header_kind: Option<HeaderKind>,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
    line_width: usize,
) -> Option<String> {
    if arguments
        .iter()
        .any(|argument| argument_has_newline(argument))
    {
        return None;
    }

    let mut line = String::from(header);
    let comment_indent = indent.chars().count() + line.chars().count();

    for (index, argument) in arguments.iter().enumerate() {
        match argument {
            Argument::InlineComment(comment) => {
                if index + 1 != arguments.len() {
                    return None;
                }
                let comment_lines =
                    comment::format_comment_lines(comment, config, comment_indent + 1, line_width);
                if comment_lines.len() != 1 {
                    return None;
                }

                let candidate = format!("{line} {}", comment_lines[0]);
                if indent.chars().count() + candidate.chars().count() > line_width {
                    return None;
                }
                line = candidate;
            }
            _ => {
                let candidate = if line.is_empty() {
                    argument.as_str().to_owned()
                } else {
                    format!("{line} {}", argument.as_str())
                };
                if indent.chars().count() + candidate.chars().count() > line_width {
                    if matches!(header_kind, Some(HeaderKind::Flag)) && arguments.len() == 1 {
                        return None;
                    }
                    return None;
                }
                line = candidate;
            }
        }
    }

    Some(line)
}

fn write_packed_arguments(
    output: &mut String,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
    line_width: usize,
) {
    let mut current = String::new();

    for (index, argument) in arguments.iter().enumerate() {
        match argument {
            Argument::InlineComment(comment) => {
                let comment_lines = comment::format_comment_lines(
                    comment,
                    config,
                    indent.chars().count(),
                    line_width,
                );
                let is_trailing_comment = index + 1 == arguments.len();
                if is_trailing_comment && comment_lines.len() == 1 && !current.is_empty() {
                    let candidate = format!("{current} {}", comment_lines[0]);
                    if indent.chars().count() + candidate.chars().count() <= line_width {
                        current = candidate;
                        continue;
                    }
                }

                flush_current_line(output, &mut current, indent);
                for line in comment_lines {
                    output.push_str(indent);
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            _ if argument_has_newline(argument) => {
                flush_current_line(output, &mut current, indent);
                write_multiline_argument(output, indent, argument.as_str());
            }
            _ => {
                let token = argument.as_str();
                let candidate = if current.is_empty() {
                    token.to_owned()
                } else {
                    format!("{current} {token}")
                };

                if current.is_empty()
                    || indent.chars().count() + candidate.chars().count() <= line_width
                {
                    current = candidate;
                } else {
                    flush_current_line(output, &mut current, indent);
                    current = token.to_owned();
                }
            }
        }
    }

    flush_current_line(output, &mut current, indent);
}

fn write_vertical_arguments(
    output: &mut String,
    arguments: &[&Argument],
    indent: &str,
    config: &Config,
) {
    for argument in arguments {
        match argument {
            Argument::InlineComment(comment) => {
                for line in comment::format_comment_lines(
                    comment,
                    config,
                    indent.chars().count(),
                    config.line_width,
                ) {
                    output.push_str(indent);
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            _ if argument_has_newline(argument) => {
                write_multiline_argument(output, indent, argument.as_str())
            }
            _ => {
                output.push_str(indent);
                output.push_str(argument.as_str());
                output.push('\n');
            }
        }
    }
}

fn write_multiline_argument(output: &mut String, indent: &str, source: &str) {
    let normalized = source.replace("\r\n", "\n");
    let mut lines = normalized.split('\n');

    output.push_str(indent);
    output.push_str(lines.next().unwrap_or_default());
    output.push('\n');

    for line in lines {
        output.push_str(line);
        output.push('\n');
    }
}

fn flush_current_line(output: &mut String, current: &mut String, indent: &str) {
    if current.is_empty() {
        return;
    }

    output.push_str(indent);
    output.push_str(current);
    output.push('\n');
    current.clear();
}

fn pack_tokens(
    prefix: &str,
    continuation: &str,
    tokens: &[String],
    line_width: usize,
    max_lines: usize,
    break_before: &[&str],
) -> Option<Vec<String>> {
    if tokens.is_empty() {
        return Some(vec![prefix.to_owned()]);
    }

    let mut lines = vec![prefix.to_owned()];

    for token in tokens {
        let normalized = token.to_ascii_uppercase();
        if break_before.contains(&normalized.as_str())
            && lines.last().is_some_and(|line| line != prefix)
            && lines.len() < max_lines
        {
            lines.push(format!("{continuation}{token}"));
            continue;
        }

        let current = lines.last_mut().expect("at least one line");
        let candidate = if current == prefix || current == continuation {
            format!("{current}{token}")
        } else {
            format!("{current} {token}")
        };

        if candidate.chars().count() <= line_width {
            *current = candidate;
            continue;
        }

        if lines.len() >= max_lines {
            return None;
        }

        lines.push(format!("{continuation}{token}"));
    }

    Some(lines)
}

fn close_multiline(
    mut lines: Vec<String>,
    base_indent: &str,
    name_len: usize,
    cmd_config: &CommandConfig<'_>,
) -> String {
    if cmd_config.dangle_parens() {
        let closer = match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => format!("{base_indent})"),
            DangleAlign::Open => format!("{base_indent}{}{})", " ".repeat(name_len), ""),
        };
        lines.push(closer);
        return lines.join("\n");
    }

    if lines.last().is_some_and(|last| last.contains('#')) {
        lines.push(format!("{base_indent})"));
        lines.join("\n")
    } else {
        if let Some(last) = lines.last_mut() {
            last.push(')');
        }
        lines.join("\n")
    }
}

fn last_output_line_has_comment(output: &str) -> bool {
    output.lines().last().is_some_and(|line| line.contains('#'))
}

fn argument_has_newline(argument: &Argument) -> bool {
    argument.as_str().contains('\n') || argument.as_str().contains('\r')
}

fn apply_case(style: CaseStyle, s: &str) -> String {
    match style {
        CaseStyle::Lower => s.to_ascii_lowercase(),
        CaseStyle::Upper => s.to_ascii_uppercase(),
        CaseStyle::Unchanged => s.to_string(),
    }
}

/// Replace leading spaces with tab characters.
fn spaces_to_tabs(output: &str, tab_size: usize) -> String {
    if tab_size == 0 {
        return output.to_string();
    }

    let mut result = String::with_capacity(output.len());
    for (i, line) in output.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }
        let leading = line.len() - line.trim_start_matches(' ').len();
        let tabs = leading / tab_size;
        let remaining = leading % tab_size;
        for _ in 0..tabs {
            result.push('\t');
        }
        for _ in 0..remaining {
            result.push(' ');
        }
        result.push_str(&line[leading..]);
    }
    result
}
