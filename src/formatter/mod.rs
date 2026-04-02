pub mod comment;
pub mod node;

use crate::config::Config;
use crate::error::Result;
use crate::parser::{self, ast::File, ast::Statement};
use crate::spec::registry::CommandRegistry;

pub fn format_source(source: &str, config: &Config) -> Result<String> {
    let file = parser::parse(source)?;
    let registry = CommandRegistry::load()?;
    format_file(&file, config, &registry)
}

pub fn format_file(file: &File, config: &Config, registry: &CommandRegistry) -> Result<String> {
    let mut output = String::new();
    let mut previous_was_content = false;
    let mut block_depth = 0usize;

    for statement in &file.statements {
        match statement {
            Statement::Command(command) => {
                block_depth = block_depth.saturating_sub(block_dedent_before(&command.name));

                if previous_was_content {
                    output.push('\n');
                }

                output.push_str(&node::format_command(
                    command,
                    config,
                    registry,
                    block_depth,
                )?);

                if let Some(trailing) = &command.trailing_comment {
                    output.push(' ');
                    output.push_str(&format_comment_text(trailing));
                }

                previous_was_content = true;
                block_depth += block_indent_after(&command.name);
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
            Statement::Comment(c) => {
                if previous_was_content {
                    output.push('\n');
                }

                output.push_str(&config.indent_str().repeat(block_depth));
                output.push_str(&format_comment_text(c));
                previous_was_content = true;
            }
        }
    }

    if !output.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}

fn format_comment_text(comment: &crate::parser::ast::Comment) -> String {
    use crate::parser::ast::Comment;
    match comment {
        Comment::Line(text) => text.clone(),
        Comment::Bracket(raw) => raw.clone(),
    }
}

fn block_dedent_before(command_name: &str) -> usize {
    match command_name.to_ascii_lowercase().as_str() {
        "elseif" | "else" | "endif" | "endforeach" | "endwhile" | "endfunction" | "endmacro"
        | "endblock" => 1,
        _ => 0,
    }
}

fn block_indent_after(command_name: &str) -> usize {
    match command_name.to_ascii_lowercase().as_str() {
        "if" | "foreach" | "while" | "function" | "macro" | "block" | "elseif" | "else" => 1,
        _ => 0,
    }
}
