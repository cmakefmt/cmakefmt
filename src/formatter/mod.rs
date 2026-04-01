pub mod node;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::parser::{self, ast::Argument, ast::Comment, ast::File, ast::Statement};
use crate::spec::registry::CommandRegistry;

pub fn format_source(source: &str, config: &Config) -> Result<String> {
    let file = parser::parse(source)?;
    let registry = CommandRegistry::load()?;
    format_file(&file, config, &registry)
}

pub fn format_file(file: &File, config: &Config, registry: &CommandRegistry) -> Result<String> {
    ensure_comment_free(file)?;

    let mut output = String::new();
    let mut previous_was_content = false;

    for statement in &file.statements {
        match statement {
            Statement::Command(command) => {
                if previous_was_content {
                    output.push('\n');
                }

                output.push_str(&node::format_command(command, config, registry)?);
                previous_was_content = true;
            }
            Statement::BlankLines(count) => {
                let newline_count = if previous_was_content {
                    count + 1
                } else {
                    *count
                };
                let newline_count = newline_count.min(config.max_empty_lines + 1);
                for _ in 0..newline_count {
                    output.push('\n');
                }
                previous_was_content = false;
            }
            Statement::Comment(comment) => {
                return Err(Error::Formatter(format!(
                    "comments are not supported in phase 3 formatter: {}",
                    comment.as_str()
                )));
            }
        }
    }

    if !output.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}

fn ensure_comment_free(file: &File) -> Result<()> {
    for statement in &file.statements {
        match statement {
            Statement::Command(command) => {
                if let Some(comment) = &command.trailing_comment {
                    return unsupported_comment(comment);
                }

                if let Some(comment) =
                    command
                        .arguments
                        .iter()
                        .find_map(|argument| match argument {
                            Argument::InlineComment(comment) => Some(comment),
                            _ => None,
                        })
                {
                    return unsupported_comment(comment);
                }
            }
            Statement::Comment(comment) => return unsupported_comment(comment),
            Statement::BlankLines(_) => {}
        }
    }

    Ok(())
}

fn unsupported_comment(comment: &Comment) -> Result<()> {
    Err(Error::Formatter(format!(
        "comments are not supported in phase 3 formatter: {}",
        comment.as_str()
    )))
}
