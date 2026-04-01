use pretty::RcDoc;

use crate::config::Config;
use crate::error::{Error, Result};
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
) -> Result<String> {
    let form = registry
        .get(&command.name)
        .form_for(first_argument(command).map(Argument::as_str));

    if command.arguments.iter().any(argument_has_newline) {
        return format_command_with_raw_multiline_args(command, form, config);
    }

    let doc = command_doc(command, form, config)?;
    Ok(doc.pretty(config.line_width).to_string())
}

fn first_argument(command: &CommandInvocation) -> Option<&Argument> {
    command
        .arguments
        .iter()
        .find(|argument| !argument.is_comment())
}

fn command_doc(command: &CommandInvocation, form: &CommandForm, config: &Config) -> Result<Doc> {
    if command.arguments.is_empty() {
        return Ok(text(command.name.clone()).append(text("()")));
    }

    let sections = split_sections(command, form)?;
    let body = RcDoc::intersperse(
        sections
            .iter()
            .map(|section| section_doc(section))
            .collect::<Result<Vec<_>>>()?,
        RcDoc::line(),
    );

    Ok(text(command.name.clone())
        .append(text("("))
        .append(
            RcDoc::line_()
                .append(body)
                .nest(config.tab_size as isize)
                .append(RcDoc::line_()),
        )
        .append(text(")"))
        .group())
}

fn split_sections<'a>(
    command: &'a CommandInvocation,
    form: &'a CommandForm,
) -> Result<Vec<Section<'a>>> {
    let mut sections = Vec::new();

    for argument in &command.arguments {
        if argument.is_comment() {
            return Err(Error::Formatter(
                "phase 3 formatter does not support comment arguments".to_owned(),
            ));
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

fn section_doc(section: &Section<'_>) -> Result<Doc> {
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
        Some(header) if section.arguments.is_empty() => text(header.to_owned()),
        Some(header) => text(header.to_owned())
            .append(RcDoc::line().append(arguments).nest(2))
            .group(),
        None => arguments,
    })
}

fn format_command_with_raw_multiline_args(
    command: &CommandInvocation,
    form: &CommandForm,
    config: &Config,
) -> Result<String> {
    let sections = split_sections(command, form)?;
    let indent = " ".repeat(config.tab_size);
    let nested_indent = " ".repeat(config.tab_size * 2);
    let mut output = String::new();

    output.push_str(&command.name);
    output.push_str("(\n");

    for section in sections {
        match section.header {
            Some(header) => {
                if section.arguments.is_empty() {
                    output.push_str(&indent);
                    output.push_str(header);
                    output.push('\n');
                    continue;
                }

                if section
                    .arguments
                    .iter()
                    .all(|argument| !argument_has_newline(argument))
                {
                    let inline = format_inline_section(Some(header), &section.arguments);
                    if indent.chars().count() + inline.chars().count() <= config.line_width {
                        output.push_str(&indent);
                        output.push_str(&inline);
                        output.push('\n');
                        continue;
                    }
                }

                output.push_str(&indent);
                output.push_str(header);
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

    output.push(')');
    Ok(output)
}

fn argument_doc(argument: &Argument) -> Result<Doc> {
    match argument {
        Argument::Bracket(bracket) => Ok(literal_doc(&bracket.raw)),
        Argument::Quoted(quoted) | Argument::Unquoted(quoted) => Ok(literal_doc(quoted)),
        Argument::InlineComment(_) => Err(Error::Formatter(
            "phase 3 formatter does not support inline comments".to_owned(),
        )),
    }
}

fn argument_has_newline(argument: &Argument) -> bool {
    argument.as_str().contains('\n') || argument.as_str().contains('\r')
}

fn format_inline_section(header: Option<&str>, arguments: &[&Argument]) -> String {
    let mut output = String::new();

    if let Some(header) = header {
        output.push_str(header);
    }

    for argument in arguments {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(argument.as_str());
    }

    output
}

fn write_raw_argument(output: &mut String, indent: &str, argument: &Argument) {
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

fn text(source: impl Into<String>) -> Doc {
    RcDoc::text(source.into())
}
