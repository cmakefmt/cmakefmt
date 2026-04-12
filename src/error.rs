// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structured error types returned by parsing, config loading, and formatting.

use std::fmt;

use pest::error::{ErrorVariant, LineColLocation};
use thiserror::Error;

/// Structured config/spec deserialization failure metadata used for
/// user-facing diagnostics.
#[derive(Debug, Clone)]
pub struct FileParseError {
    /// Parser format name, such as `TOML` or `YAML`.
    pub format: &'static str,
    /// Human-readable parser message.
    pub message: Box<str>,
    /// Optional 1-based line number.
    pub line: Option<usize>,
    /// Optional 1-based column number.
    pub column: Option<usize>,
}

/// Crate-owned parser diagnostics used by [`Error`] without exposing `pest`
/// internals in the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    /// Human-readable parser detail.
    pub message: Box<str>,
    /// 1-based source line number.
    pub line: usize,
    /// 1-based source column number.
    pub column: usize,
}

impl ParseDiagnostic {
    pub(crate) fn from_pest(error: &pest::error::Error<crate::parser::Rule>) -> Self {
        let (line, column) = match error.line_col {
            LineColLocation::Pos((line, column)) => (line, column),
            LineColLocation::Span((line, column), _) => (line, column),
        };
        let message = match &error.variant {
            ErrorVariant::ParsingError { positives, .. } if !positives.is_empty() => format!(
                "expected {}",
                positives
                    .iter()
                    .map(|rule| format!("{rule:?}").replace('_', " "))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ErrorVariant::CustomError { message } => message.clone(),
            _ => error.to_string(),
        };
        Self {
            message: message.into_boxed_str(),
            line,
            column,
        }
    }
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Errors that can be returned by parsing, config loading, spec loading, or
/// formatting operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A parser error annotated with source text and line-offset context.
    #[error("parse error in {display_name}: {diagnostic}")]
    ParseContext {
        /// Human-facing source name, for example a path or `<stdin>`.
        display_name: String,
        /// The source text that failed to parse.
        source_text: Box<str>,
        /// The 1-based source line number where this parser chunk started.
        start_line: usize,
        /// Whether earlier barrier/fence handling affected how this chunk was parsed.
        barrier_context: bool,
        /// Structured parser diagnostic.
        diagnostic: ParseDiagnostic,
    },

    /// A user config parse error.
    #[error("config error in {path}: {source_message}")]
    Config {
        /// The config file that failed to deserialize.
        path: std::path::PathBuf,
        /// Structured parser details for the failure.
        details: FileParseError,
        /// Cached display string used by `thiserror`.
        source_message: Box<str>,
    },

    /// A built-in or user override spec parse error.
    #[error("spec error in {path}: {source_message}")]
    Spec {
        /// The spec file that failed to deserialize.
        path: std::path::PathBuf,
        /// Structured parser details for the failure.
        details: FileParseError,
        /// Cached display string used by `thiserror`.
        source_message: Box<str>,
    },

    /// A filesystem or stream I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A higher-level formatter or CLI error that does not fit another
    /// structured variant.
    #[error("formatter error: {0}")]
    Formatter(String),

    /// A formatted line exceeded the configured line width and
    /// `require_valid_layout` is enabled.
    #[error(
        "line {line_no} is {width} characters wide, exceeding the configured limit of {limit}"
    )]
    LayoutTooWide {
        /// 1-based line number in the formatted output.
        line_no: usize,
        /// Actual character width of the offending line.
        width: usize,
        /// Configured [`Config::line_width`] limit.
        limit: usize,
    },
}

/// Convenience alias for crate-level results.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Attach a human-facing source name to a contextual parser error.
    pub fn with_display_name(self, display_name: impl Into<String>) -> Self {
        match self {
            Self::ParseContext {
                source_text,
                start_line,
                barrier_context,
                diagnostic,
                ..
            } => Self::ParseContext {
                display_name: display_name.into(),
                source_text,
                start_line,
                barrier_context,
                diagnostic,
            },
            other => other,
        }
    }
}
