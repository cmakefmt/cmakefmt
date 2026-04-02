pub mod comment;
pub mod node;

use crate::config::Config;
use crate::error::Result;
use crate::parser::{self, ast::File, ast::Statement};
use crate::spec::registry::CommandRegistry;

pub fn format_source(source: &str, config: &Config) -> Result<String> {
    format_source_with_registry(source, config, CommandRegistry::builtins())
}

pub fn format_source_with_debug(source: &str, config: &Config) -> Result<(String, Vec<String>)> {
    format_source_with_registry_debug(source, config, CommandRegistry::builtins())
}

pub fn format_source_with_registry(
    source: &str,
    config: &Config,
    registry: &CommandRegistry,
) -> Result<String> {
    Ok(format_source_impl(source, config, registry, &mut DebugLog::disabled())?.0)
}

pub fn format_source_with_registry_debug(
    source: &str,
    config: &Config,
    registry: &CommandRegistry,
) -> Result<(String, Vec<String>)> {
    let mut lines = Vec::new();
    let mut debug = DebugLog::enabled(&mut lines);
    let (formatted, _) = format_source_impl(source, config, registry, &mut debug)?;
    Ok((formatted, lines))
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
                    let comment_lines =
                        comment::format_comment_lines(trailing, config, 0, config.line_width);
                    if let Some(first) = comment_lines.first() {
                        output.push(' ');
                        output.push_str(first);
                    }
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

                let indent = config.indent_str().repeat(block_depth);
                let comment_lines = comment::format_comment_lines(
                    c,
                    config,
                    indent.chars().count(),
                    config.line_width,
                );
                for (index, line) in comment_lines.iter().enumerate() {
                    if index > 0 {
                        output.push('\n');
                    }
                    output.push_str(&indent);
                    output.push_str(line);
                }
                previous_was_content = true;
            }
        }
    }

    if !output.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}

fn format_source_impl(
    source: &str,
    config: &Config,
    registry: &CommandRegistry,
    debug: &mut DebugLog<'_>,
) -> Result<(String, usize)> {
    let mut output = String::new();
    let mut enabled_chunk = String::new();
    let mut total_statements = 0usize;
    let mut mode = BarrierMode::Enabled;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let line_no = line_index + 1;
        match detect_barrier(line) {
            Some(BarrierEvent::DisableByDirective(kind)) => {
                let statements =
                    flush_enabled_chunk(&mut output, &mut enabled_chunk, config, registry, debug)?;
                total_statements += statements;
                debug.log(format!(
                    "formatter: disabled formatting at line {line_no} via {kind}: off"
                ));
                output.push_str(line);
                mode = BarrierMode::DisabledByDirective;
            }
            Some(BarrierEvent::EnableByDirective(kind)) => {
                let statements =
                    flush_enabled_chunk(&mut output, &mut enabled_chunk, config, registry, debug)?;
                total_statements += statements;
                debug.log(format!(
                    "formatter: enabled formatting at line {line_no} via {kind}: on"
                ));
                output.push_str(line);
                if matches!(mode, BarrierMode::DisabledByDirective) {
                    mode = BarrierMode::Enabled;
                }
            }
            Some(BarrierEvent::Fence) => {
                let statements =
                    flush_enabled_chunk(&mut output, &mut enabled_chunk, config, registry, debug)?;
                total_statements += statements;
                let next_mode = if matches!(mode, BarrierMode::DisabledByFence) {
                    BarrierMode::Enabled
                } else {
                    BarrierMode::DisabledByFence
                };
                debug.log(format!(
                    "formatter: toggled fence region at line {line_no} -> {}",
                    next_mode.as_str()
                ));
                output.push_str(line);
                mode = next_mode;
            }
            None => {
                if matches!(mode, BarrierMode::Enabled) {
                    enabled_chunk.push_str(line);
                } else {
                    output.push_str(line);
                }
            }
        }
    }

    total_statements +=
        flush_enabled_chunk(&mut output, &mut enabled_chunk, config, registry, debug)?;
    Ok((output, total_statements))
}

fn flush_enabled_chunk(
    output: &mut String,
    enabled_chunk: &mut String,
    config: &Config,
    registry: &CommandRegistry,
    debug: &mut DebugLog<'_>,
) -> Result<usize> {
    if enabled_chunk.is_empty() {
        return Ok(0);
    }

    let file = parser::parse(enabled_chunk)?;
    let statement_count = file.statements.len();
    debug.log(format!(
        "formatter: formatting enabled chunk with {statement_count} statement(s)"
    ));
    let formatted = format_file(&file, config, registry)?;
    output.push_str(&formatted);
    enabled_chunk.clear();
    Ok(statement_count)
}

fn detect_barrier(line: &str) -> Option<BarrierEvent<'_>> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    let body = trimmed[1..].trim_start().trim_end();
    if body.starts_with("~~~") {
        return Some(BarrierEvent::Fence);
    }

    for kind in ["cmake-format", "cmakefmt"] {
        if body == format!("{kind}: off") {
            return Some(BarrierEvent::DisableByDirective(kind));
        }
        if body == format!("{kind}: on") {
            return Some(BarrierEvent::EnableByDirective(kind));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarrierMode {
    Enabled,
    DisabledByDirective,
    DisabledByFence,
}

impl BarrierMode {
    fn as_str(self) -> &'static str {
        match self {
            BarrierMode::Enabled => "enabled",
            BarrierMode::DisabledByDirective => "disabled-by-directive",
            BarrierMode::DisabledByFence => "disabled-by-fence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarrierEvent<'a> {
    DisableByDirective(&'a str),
    EnableByDirective(&'a str),
    Fence,
}

struct DebugLog<'a> {
    lines: Option<&'a mut Vec<String>>,
}

impl<'a> DebugLog<'a> {
    fn disabled() -> Self {
        Self { lines: None }
    }

    fn enabled(lines: &'a mut Vec<String>) -> Self {
        Self { lines: Some(lines) }
    }

    fn log(&mut self, message: impl Into<String>) {
        if let Some(lines) = self.lines.as_deref_mut() {
            lines.push(message.into());
        }
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
