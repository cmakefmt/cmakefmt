use pretty::RcDoc;

use crate::config::{CaseStyle, CommandConfig, Config, DangleAlign};
use crate::error::Result;
use crate::formatter::comment;
use crate::parser::ast::{Argument, CommandInvocation};
use crate::spec::registry::CommandRegistry;
use crate::spec::CommandForm;

type Doc = RcDoc<'static, ()>;

#[derive(Debug)]
struct Section<'a> {
    header: Option<&'a str>,
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

    let has_inline_comments = command.arguments.iter().any(Argument::is_comment);
    let force_raw = has_inline_comments
        || command.arguments.iter().any(argument_has_newline)
        || cmd_config.dangle_align() == DangleAlign::Open;

    let output = if force_raw {
        format_command_with_raw_multiline_args(command, form, &cmd_config, block_depth)?
    } else {
        let doc = command_doc(command, form, &cmd_config)?;
        indent_pretty_output(
            &doc.pretty(cmd_config.line_width()).to_string(),
            &cmd_config.indent_str().repeat(block_depth),
        )
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

fn command_doc(
    command: &CommandInvocation,
    form: &CommandForm,
    cmd_config: &CommandConfig<'_>,
) -> Result<Doc> {
    let name = format_name(command, cmd_config);

    if command.arguments.is_empty() {
        return Ok(text(name).append(text("()")));
    }

    let sections = split_sections(command, form)?;
    let body = RcDoc::intersperse(
        sections
            .iter()
            .map(|section| section_doc(section, cmd_config))
            .collect::<Result<Vec<_>>>()?,
        RcDoc::line(),
    );

    let tab = cmd_config.tab_size() as isize;

    if cmd_config.dangle_parens() {
        // Closing paren on its own line when broken
        Ok(text(name)
            .append(text("("))
            .append(RcDoc::line_().append(body).nest(tab).append(RcDoc::line_()))
            .append(text(")"))
            .group())
    } else {
        // Closing paren on same line as last argument when broken
        Ok(text(name)
            .append(text("("))
            .append(RcDoc::line_().append(body).nest(tab))
            .append(text(")"))
            .group())
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
        let is_header = form.kwargs.contains_key(&normalized) || form.flags.contains(&normalized);

        if is_header {
            sections.push(Section {
                header: Some(token),
                arguments: Vec::new(),
            });
            continue;
        }

        if sections.is_empty() {
            sections.push(Section {
                header: None,
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

fn section_doc(section: &Section<'_>, cmd_config: &CommandConfig<'_>) -> Result<Doc> {
    let arguments = RcDoc::intersperse(
        section
            .arguments
            .iter()
            .map(|argument| argument_doc(argument))
            .collect::<Result<Vec<_>>>()?,
        RcDoc::line(),
    )
    .group();

    Ok(match section.header {
        Some(header) if section.arguments.is_empty() => {
            text(apply_case(cmd_config.keyword_case(), header))
        }
        Some(header) => text(apply_case(cmd_config.keyword_case(), header))
            .append(RcDoc::line().append(arguments).nest(2))
            .group(),
        None => arguments,
    })
}

fn format_command_with_raw_multiline_args(
    command: &CommandInvocation,
    form: &CommandForm,
    cmd_config: &CommandConfig<'_>,
    block_depth: usize,
) -> Result<String> {
    let sections = split_sections(command, form)?;
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
            Some(header) => {
                let header = apply_case(cmd_config.keyword_case(), header);
                if section.arguments.is_empty() {
                    output.push_str(&indent);
                    output.push_str(&header);
                    output.push('\n');
                    continue;
                }

                if !section_has_comment(&section)
                    && section
                        .arguments
                        .iter()
                        .all(|argument| !argument_has_newline(argument))
                {
                    let inline = format_inline_section(Some(&header), &section.arguments);
                    if indent.chars().count() + inline.chars().count() <= cmd_config.line_width() {
                        output.push_str(&indent);
                        output.push_str(&inline);
                        output.push('\n');
                        continue;
                    }
                }

                output.push_str(&indent);
                output.push_str(&header);
                output.push('\n');
                for argument in section.arguments {
                    write_raw_argument(&mut output, &nested_indent, argument);
                }
            }
            None => {
                for argument in section.arguments {
                    write_raw_argument(&mut output, &indent, argument);
                }
            }
        }
    }

    if cmd_config.dangle_parens() {
        match cmd_config.dangle_align() {
            DangleAlign::Prefix | DangleAlign::Close => {
                output.push_str(&base_indent);
                output.push(')');
            }
            DangleAlign::Open => {
                output.push_str(&base_indent);
                let paren_col = name.len();
                for _ in 0..paren_col {
                    output.push(' ');
                }
                output.push(')');
            }
        }
    } else {
        // Non-dangle: put ) on the last line
        if output.ends_with('\n') {
            output.pop();
        }
        output.push(')');
    }
    Ok(output)
}

fn argument_doc(argument: &Argument) -> Result<Doc> {
    match argument {
        Argument::Bracket(bracket) => Ok(literal_doc(&bracket.raw)),
        Argument::Quoted(quoted) | Argument::Unquoted(quoted) => Ok(literal_doc(quoted)),
        Argument::InlineComment(c) => Ok(comment::inline_comment_doc(c)),
    }
}

fn argument_has_newline(argument: &Argument) -> bool {
    argument.as_str().contains('\n') || argument.as_str().contains('\r')
}

fn section_has_comment(section: &Section<'_>) -> bool {
    section.arguments.iter().any(|a| a.is_comment())
}

fn format_inline_section(header: Option<&str>, arguments: &[&Argument]) -> String {
    let mut output = String::new();

    if let Some(header) = header {
        output.push_str(header);
    }

    for argument in arguments {
        if argument.is_comment() {
            continue;
        }
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(argument.as_str());
    }

    output
}

fn write_raw_argument(output: &mut String, indent: &str, argument: &Argument) {
    if let Argument::InlineComment(c) = argument {
        output.push_str(indent);
        output.push_str(&super::format_comment_text(c));
        output.push('\n');
        return;
    }

    let normalized = argument.as_str().replace("\r\n", "\n");
    let mut lines = normalized.split('\n');

    output.push_str(indent);
    output.push_str(lines.next().unwrap_or_default());
    output.push('\n');

    for line in lines {
        output.push_str(line);
        output.push('\n');
    }
}

fn literal_doc(source: &str) -> Doc {
    let normalized = source.replace("\r\n", "\n");
    let mut parts = normalized.split('\n');
    let first = text(parts.next().unwrap_or_default().to_owned());

    parts.fold(first, |doc, part| {
        doc.append(RcDoc::hardline()).append(text(part.to_owned()))
    })
}

fn indent_pretty_output(output: &str, indent: &str) -> String {
    if indent.is_empty() {
        return output.to_string();
    }

    let mut indented = String::with_capacity(output.len() + indent.len());
    for (index, line) in output.split('\n').enumerate() {
        if index > 0 {
            indented.push('\n');
        }

        if !line.is_empty() {
            indented.push_str(indent);
        }
        indented.push_str(line);
    }

    indented
}

fn text(source: impl Into<String>) -> Doc {
    RcDoc::text(source.into())
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
