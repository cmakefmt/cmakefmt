// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Top-level formatter entry points.
//!
//! These functions parse input, apply barrier handling, and render a formatted
//! output string using the command registry and runtime configuration.
//!
//! # Format-barrier directives
//!
//! The source-string entry points ([`format_source`],
//! [`format_source_with_registry`], [`format_source_with_debug`],
//! [`format_source_with_registry_debug`]) scan each input line for
//! *barrier directives* that toggle formatting on and off:
//!
//! | Directive | Effect |
//! |-----------|--------|
//! | `# cmake-format: off` / `# cmake-format: on` | Skip / resume formatting |
//! | `# cmakefmt: off` / `# cmakefmt: on` | Same, cmakefmt-branded |
//! | `# fmt: off` / `# fmt: on` | Generic alias |
//! | `# ~~~` (matched pair) | Fence region — content between fences is emitted verbatim |
//!
//! Leading whitespace before the `#` is allowed. Lines inside a
//! disabled region are passed through unchanged.
//!
//! Note: [`format_parsed_file`] does **not** honour these directives
//! — barrier detection happens pre-parse, so if you have the AST
//! already you've bypassed that step. Use a source-string entry
//! point if you need barriers.

pub(crate) mod comment;
pub(crate) mod node;

// `dump.rs` is the only consumer of these re-exports and is itself
// `#[cfg(feature = "cli")]`. Without the gate this `use` warns under
// `--no-default-features` and `--features lsp` builds.
#[cfg(feature = "cli")]
pub(crate) use node::{split_sections, HeaderKind};

use std::path::PathBuf;

use crate::config::{Config, LineEnding};
use crate::error::{Error, FileParseError, Result};
use crate::parser::{self, ast::File, ast::Statement};
use crate::spec::registry::CommandRegistry;

/// Format raw CMake source using the built-in command registry.
///
/// The output always ends with a newline. When
/// [`Config::line_ending`] is [`LineEnding::Auto`], the output line
/// ending is detected from the input (CRLF if the source contains
/// any `\r\n`, otherwise LF).
///
/// # Examples
///
/// ```
/// use cmakefmt::{format_source, Config};
///
/// let cmake = "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n";
/// let formatted = format_source(cmake, &Config::default()).unwrap();
/// assert_eq!(formatted, "cmake_minimum_required(VERSION 3.20)\n");
/// ```
pub fn format_source(source: &str, config: &Config) -> Result<String> {
    format_source_with_registry(source, config, CommandRegistry::builtins())
}

/// Format raw CMake source using the built-in registry and also return debug
/// lines describing the formatter's decisions.
///
/// The returned `Vec<String>` contains one human-readable log line
/// per formatting decision (layout choice, section split, fallback
/// paths, barrier events). The exact wording is **unstable across
/// releases** and intended for interactive debugging and bug
/// reports, not programmatic consumption.
pub fn format_source_with_debug(source: &str, config: &Config) -> Result<(String, Vec<String>)> {
    format_source_with_registry_debug(source, config, CommandRegistry::builtins())
}

/// Format raw CMake source using an explicit command registry.
///
/// Use this when you need a registry that merges the built-ins with a user
/// override file.
///
/// # Examples
///
/// ```
/// use cmakefmt::{format_source_with_registry, Config, CommandRegistry};
///
/// let registry = CommandRegistry::from_builtins_and_overrides(
///     None::<&std::path::Path>,
/// ).unwrap();
/// let cmake = "TARGET_LINK_LIBRARIES(mylib PUBLIC dep1)\n";
/// let formatted = format_source_with_registry(
///     cmake, &Config::default(), &registry,
/// ).unwrap();
/// assert_eq!(formatted, "target_link_libraries(mylib PUBLIC dep1)\n");
/// ```
pub fn format_source_with_registry(
    source: &str,
    config: &Config,
    registry: &CommandRegistry,
) -> Result<String> {
    if config.disable {
        return Ok(source.to_owned());
    }
    validate_runtime_config(config)?;
    let formatted = format_source_impl(source, config, registry, &mut DebugLog::disabled())?.0;
    Ok(apply_line_ending(source, &formatted, config.line_ending))
}

/// Format raw CMake source using an explicit registry and return debug output.
pub fn format_source_with_registry_debug(
    source: &str,
    config: &Config,
    registry: &CommandRegistry,
) -> Result<(String, Vec<String>)> {
    if config.disable {
        return Ok((source.to_owned(), Vec::new()));
    }
    validate_runtime_config(config)?;
    let mut lines = Vec::new();
    let mut debug = DebugLog::enabled(&mut lines);
    let (formatted, _) = format_source_impl(source, config, registry, &mut debug)?;
    Ok((
        apply_line_ending(source, &formatted, config.line_ending),
        lines,
    ))
}

/// Format an already parsed AST file using the original source text.
///
/// This entry point preserves the same high-level config semantics as
/// [`format_source_with_registry`]: `disable` returns the original `source`
/// unchanged and `line_ending` is applied relative to the original source.
///
/// Useful when you want to parse once and format the same AST repeatedly with
/// different [`Config`] or registry settings, avoiding re-parsing overhead.
///
/// # Caveat: no barrier handling
///
/// Unlike [`format_source`] and its siblings, this function does
/// **not** honour `# cmake-format: off/on`, `# cmakefmt: off/on`,
/// `# fmt: off/on`, or `# ~~~` fence regions. Barrier detection
/// happens pre-parse in the source-string pipeline, so by the time
/// you hand in a parsed AST the opportunity has passed. Use one of
/// the source-string entry points if your input contains barrier
/// directives.
///
/// # Examples
///
/// ```
/// use cmakefmt::{format_parsed_file, Config, CommandRegistry};
///
/// let cmake = "PROJECT(MyProject)\n";
/// let file = cmakefmt::parser::parse(cmake).unwrap();
/// let formatted = format_parsed_file(
///     cmake,
///     &file,
///     &Config::default(),
///     CommandRegistry::builtins(),
/// ).unwrap();
/// assert_eq!(formatted, "project(MyProject)\n");
/// ```
pub fn format_parsed_file(
    source: &str,
    file: &File,
    config: &Config,
    registry: &CommandRegistry,
) -> Result<String> {
    if config.disable {
        return Ok(source.to_owned());
    }
    validate_runtime_config(config)?;
    let formatted =
        format_parsed_file_with_debug(file, config, registry, &mut DebugLog::disabled())?;
    Ok(apply_line_ending(source, &formatted, config.line_ending))
}

fn format_parsed_file_with_debug(
    file: &File,
    config: &Config,
    registry: &CommandRegistry,
    debug: &mut DebugLog<'_>,
) -> Result<String> {
    let patterns = config.compiled_patterns().map_err(runtime_config_error)?;
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
                    &patterns,
                    registry,
                    block_depth,
                    debug,
                )?);

                if let Some(trailing) = &command.trailing_comment {
                    let comment_indent_width = output
                        .rsplit('\n')
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .count()
                        + 1;
                    let comment_lines = comment::format_comment_lines(
                        trailing,
                        config,
                        &patterns,
                        comment_indent_width,
                        config.line_width,
                    );
                    if let Some((first, rest)) = comment_lines.split_first() {
                        output.push(' ');
                        output.push_str(first);
                        let continuation_indent = " ".repeat(comment_indent_width);
                        for line in rest {
                            output.push('\n');
                            output.push_str(&continuation_indent);
                            output.push_str(line);
                        }
                    }
                }

                previous_was_content = true;
                block_depth += block_indent_after(&command.name);
            }
            Statement::TemplatePlaceholder(placeholder) => {
                if previous_was_content {
                    output.push('\n');
                }

                output.push_str(placeholder);
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
            Statement::Comment(c) => {
                if previous_was_content {
                    output.push('\n');
                }

                let indent = config.indent_str().repeat(block_depth);
                let comment_lines = comment::format_comment_lines(
                    c,
                    config,
                    &patterns,
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

    if config.require_valid_layout {
        for (i, line) in output.split('\n').enumerate() {
            // Skip the final empty string produced by the trailing newline.
            if line.is_empty() {
                continue;
            }
            let width = line.chars().count();
            if width > config.line_width {
                return Err(Error::LayoutTooWide {
                    line_no: i + 1,
                    width,
                    limit: config.line_width,
                });
            }
        }
    }

    Ok(output)
}

/// Apply the configured line-ending style to `formatted` output.
///
/// The formatter always emits LF internally. `source` is consulted when
/// `line_ending` is [`LineEnding::Auto`] to detect the predominant style.
fn apply_line_ending(source: &str, formatted: &str, line_ending: LineEnding) -> String {
    let use_crlf = match line_ending {
        LineEnding::Unix => false,
        LineEnding::Windows => true,
        LineEnding::Auto => {
            // Detect from input: if any \r\n is present, assume CRLF.
            source.contains("\r\n")
        }
    };
    if use_crlf {
        formatted.replace('\n', "\r\n")
    } else {
        formatted.to_owned()
    }
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
    let mut enabled_chunk_start_line = 1usize;
    let mut saw_barrier = false;

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let line_no = line_index + 1;
        match detect_barrier(line) {
            Some(BarrierEvent::DisableByDirective(kind)) => {
                let statements = flush_enabled_chunk(
                    &mut output,
                    &mut enabled_chunk,
                    config,
                    registry,
                    debug,
                    enabled_chunk_start_line,
                    saw_barrier,
                )?;
                total_statements += statements;
                debug.log(format!(
                    "formatter: disabled formatting at line {line_no} via {kind}: off"
                ));
                output.push_str(line);
                mode = BarrierMode::DisabledByDirective;
                saw_barrier = true;
            }
            Some(BarrierEvent::EnableByDirective(kind)) => {
                let statements = flush_enabled_chunk(
                    &mut output,
                    &mut enabled_chunk,
                    config,
                    registry,
                    debug,
                    enabled_chunk_start_line,
                    saw_barrier,
                )?;
                total_statements += statements;
                debug.log(format!(
                    "formatter: enabled formatting at line {line_no} via {kind}: on"
                ));
                output.push_str(line);
                if matches!(mode, BarrierMode::DisabledByDirective) {
                    mode = BarrierMode::Enabled;
                }
                saw_barrier = true;
            }
            Some(BarrierEvent::Fence) => {
                let statements = flush_enabled_chunk(
                    &mut output,
                    &mut enabled_chunk,
                    config,
                    registry,
                    debug,
                    enabled_chunk_start_line,
                    saw_barrier,
                )?;
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
                saw_barrier = true;
            }
            None => {
                if matches!(mode, BarrierMode::Enabled) {
                    if enabled_chunk.is_empty() {
                        enabled_chunk_start_line = line_no;
                    }
                    enabled_chunk.push_str(line);
                } else {
                    output.push_str(line);
                }
            }
        }
    }

    total_statements += flush_enabled_chunk(
        &mut output,
        &mut enabled_chunk,
        config,
        registry,
        debug,
        enabled_chunk_start_line,
        saw_barrier,
    )?;
    Ok((output, total_statements))
}

fn flush_enabled_chunk(
    output: &mut String,
    enabled_chunk: &mut String,
    config: &Config,
    registry: &CommandRegistry,
    debug: &mut DebugLog<'_>,
    chunk_start_line: usize,
    barrier_context: bool,
) -> Result<usize> {
    if enabled_chunk.is_empty() {
        return Ok(0);
    }

    let file = match parser::parse(enabled_chunk) {
        Ok(file) => file,
        Err(Error::Parse(parse_error)) => {
            let _ = barrier_context;
            return Err(Error::Parse(crate::error::ParseError {
                display_name: "<source>".to_owned(),
                source_text: enabled_chunk.clone().into_boxed_str(),
                start_line: chunk_start_line,
                diagnostic: parse_error.diagnostic,
            }));
        }
        Err(err) => return Err(err),
    };
    let statement_count = file.statements.len();
    debug.log(format!(
        "formatter: formatting enabled chunk with {statement_count} statement(s) starting at source line {chunk_start_line}"
    ));
    let formatted = format_parsed_file_with_debug(&file, config, registry, debug)?;
    output.push_str(&formatted);
    enabled_chunk.clear();
    Ok(statement_count)
}

fn validate_runtime_config(config: &Config) -> Result<()> {
    config.validate_patterns().map_err(runtime_config_error)?;
    Ok(())
}

fn runtime_config_error(message: String) -> Error {
    Error::Config(crate::error::ConfigError {
        path: PathBuf::from("<programmatic-config>"),
        details: FileParseError {
            format: "runtime",
            message: message.into_boxed_str(),
            line: None,
            column: None,
        },
    })
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

    if body == "cmake-format: off" {
        return Some(BarrierEvent::DisableByDirective("cmake-format"));
    }
    if body == "cmake-format: on" {
        return Some(BarrierEvent::EnableByDirective("cmake-format"));
    }
    if body == "cmakefmt: off" {
        return Some(BarrierEvent::DisableByDirective("cmakefmt"));
    }
    if body == "cmakefmt: on" {
        return Some(BarrierEvent::EnableByDirective("cmakefmt"));
    }
    if body == "fmt: off" {
        return Some(BarrierEvent::DisableByDirective("fmt"));
    }
    if body == "fmt: on" {
        return Some(BarrierEvent::EnableByDirective("fmt"));
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

pub(crate) struct DebugLog<'a> {
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
    usize::from(matches_ascii_insensitive(
        command_name,
        &[
            "elseif",
            "else",
            "endif",
            "endforeach",
            "endwhile",
            "endfunction",
            "endmacro",
            "endblock",
        ],
    ))
}

fn block_indent_after(command_name: &str) -> usize {
    usize::from(matches_ascii_insensitive(
        command_name,
        &[
            "if", "foreach", "while", "function", "macro", "block", "elseif", "else",
        ],
    ))
}

fn matches_ascii_insensitive(input: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| input.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_parsed_file_honors_disable() {
        let source = "set(  X  1 )\n";
        let file = parser::parse(source).unwrap();
        let config = Config {
            disable: true,
            ..Config::default()
        };

        let formatted =
            format_parsed_file(source, &file, &config, CommandRegistry::builtins()).unwrap();

        assert_eq!(formatted, source);
    }

    #[test]
    fn format_parsed_file_applies_line_endings_relative_to_source() {
        let source = "set(  X  1 )\r\n";
        let file = parser::parse(source).unwrap();
        let config = Config {
            line_ending: LineEnding::Auto,
            ..Config::default()
        };

        let formatted =
            format_parsed_file(source, &file, &config, CommandRegistry::builtins()).unwrap();

        assert_eq!(formatted, "set(X 1)\r\n");
    }

    #[test]
    fn format_source_rejects_invalid_programmatic_regex_config() {
        let config = Config {
            fence_pattern: "[".to_owned(),
            ..Config::default()
        };

        let err = format_source("set(X 1)\n", &config).unwrap_err();
        match err {
            Error::Config(config_err) => {
                assert_eq!(config_err.path, PathBuf::from("<programmatic-config>"));
                assert_eq!(config_err.details.format, "runtime");
                assert!(config_err.details.message.contains("invalid regex"));
            }
            other => panic!("expected config error, got {other:?}"),
        }
    }
}
