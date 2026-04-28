// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structured error types returned by parsing, config loading, and formatting.
//!
//! Every fallible crate API returns [`Result`], which is
//! `std::result::Result<T, Error>`. The [`enum@Error`] enum
//! distinguishes sources:
//!
//! - [`Error::Parse`] — CMake source failed to parse; line/column
//!   info is 1-based.
//! - [`Error::Config`] — a `.cmakefmt.yaml|yml|toml` (or
//!   `from_yaml_str` input) failed to deserialise, or a programmatic
//!   [`crate::Config`] had an invalid regex pattern.
//! - [`Error::Spec`] — a `commands:` override file (or string)
//!   failed to deserialise, or the built-in spec file itself did.
//! - [`Error::Io`] — filesystem or stream I/O failure.
//! - [`Error::Formatter`] — higher-level formatter or CLI failure
//!   that doesn't fit another variant.
//! - [`Error::LayoutTooWide`] — *only* produced when
//!   [`crate::Config::require_valid_layout`] is enabled and a
//!   formatted line exceeded the configured width. Not a bug in the
//!   formatter — a signal to the caller.
//!
//! [`crate::error::FileParseError`] and
//! [`crate::error::ParseDiagnostic`] carry structured line/column
//! metadata (1-based, both) so editor integrations can point at the
//! offending source without re-parsing the error string.

use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

/// Structured config/spec deserialization failure metadata used for
/// user-facing diagnostics.
///
/// When present, `line` and `column` are **1-based** (not 0-based),
/// matching the convention used by editors and the `ParseDiagnostic`
/// counterpart for CMake source errors.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

impl fmt::Display for FileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Crate-owned parser diagnostics used by [`enum@Error`] without exposing `pest`
/// internals in the public API.
///
/// `line` and `column` are **1-based** and count columns by characters
/// (not bytes), so multi-byte UTF-8 characters occupy a single
/// column.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseDiagnostic {
    /// Human-readable parser detail.
    pub message: Box<str>,
    /// 1-based source line number.
    pub line: usize,
    /// 1-based source column number.
    pub column: usize,
}

impl fmt::Display for ParseDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Stable parse error returned by the public library API.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("parse error in {display_name}: {diagnostic}")]
#[non_exhaustive]
pub struct ParseError {
    /// Human-facing source name, for example a path or `<stdin>`.
    pub display_name: String,
    /// The source text that failed to parse.
    pub source_text: Box<str>,
    /// The 1-based source line number where this parser chunk started.
    pub start_line: usize,
    /// Structured parser diagnostic.
    pub diagnostic: ParseDiagnostic,
}

impl ParseError {
    fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }
}

/// Stable config-file parse error returned by the public library API.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("config error in {path}: {details}")]
#[non_exhaustive]
pub struct ConfigError {
    /// The config file that failed to deserialize.
    pub path: PathBuf,
    /// Structured parser details for the failure.
    pub details: FileParseError,
}

impl ConfigError {
    /// Build a `ConfigError` from its component parts. Used at the
    /// many call sites that wrap `serde` parser errors into the
    /// crate's structured error type.
    pub(crate) fn new(
        path: PathBuf,
        format: &'static str,
        message: impl Into<Box<str>>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        Self {
            path,
            details: FileParseError {
                format,
                message: message.into(),
                line,
                column,
            },
        }
    }
}

/// Stable command-spec parse error returned by the public library API.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("spec error in {path}: {details}")]
#[non_exhaustive]
pub struct SpecError {
    /// The spec file that failed to deserialize.
    pub path: PathBuf,
    /// Structured parser details for the failure.
    pub details: FileParseError,
}

impl SpecError {
    /// Build a `SpecError` from its component parts. Mirror of
    /// [`ConfigError::new`] for spec-file parse failures.
    pub(crate) fn new(
        path: PathBuf,
        format: &'static str,
        message: impl Into<Box<str>>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        Self {
            path,
            details: FileParseError {
                format,
                message: message.into(),
                line,
                column,
            },
        }
    }
}

/// Errors that can be returned by parsing, config loading, spec loading, or
/// formatting operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A parser error annotated with source text and line-offset context.
    #[error("{0}")]
    Parse(#[from] ParseError),

    /// A user config parse error.
    #[error("{0}")]
    Config(#[from] ConfigError),

    /// A built-in or user override spec parse error.
    #[error("{0}")]
    Spec(#[from] SpecError),

    /// A filesystem or stream I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A higher-level formatter or CLI error that does not fit another
    /// structured variant. In practice this covers: runtime regex
    /// validation failures on a programmatically-built [`crate::Config`]
    /// (before the config is saved to disk), CLI argument validation
    /// failures, and rendering failures in the config/spec pretty-printers.
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
        /// Configured [`crate::Config::line_width`] limit.
        limit: usize,
    },
}

/// Convenience alias for crate-level results.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Attach a human-facing source name (e.g. a file path) to a
    /// contextual [`ParseError`]. No-op for any other variant —
    /// `Config`, `Spec`, `Io`, `Formatter`, and `LayoutTooWide`
    /// already carry the context they need and are returned
    /// unchanged.
    pub fn with_display_name(self, display_name: impl Into<String>) -> Self {
        match self {
            Self::Parse(parse) => Self::Parse(parse.with_display_name(display_name)),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diagnostic_display_shows_message() {
        let diag = ParseDiagnostic {
            message: "expected argument part".into(),
            line: 5,
            column: 10,
        };
        assert_eq!(diag.to_string(), "expected argument part");
    }

    #[test]
    fn parse_diagnostic_from_parse_error() {
        let source = "if(\n";
        let err = crate::parser::parse(source).unwrap_err();
        if let Error::Parse(ParseError { diagnostic, .. }) = err {
            assert!(diagnostic.line >= 1);
            assert!(diagnostic.column >= 1);
            assert!(!diagnostic.message.is_empty());
        } else {
            panic!("expected Parse, got {err:?}");
        }
    }

    #[test]
    fn error_parse_display() {
        let err = Error::Parse(ParseError {
            display_name: "test.cmake".to_owned(),
            source_text: "if(\n".into(),
            start_line: 1,
            diagnostic: ParseDiagnostic {
                message: "expected argument part".into(),
                line: 1,
                column: 4,
            },
        });
        let msg = err.to_string();
        assert!(msg.contains("test.cmake"));
        assert!(msg.contains("expected argument part"));
    }

    #[test]
    fn error_config_display() {
        let err = Error::Config(ConfigError {
            path: std::path::PathBuf::from("bad.yaml"),
            details: FileParseError {
                format: "YAML",
                message: "unexpected key".into(),
                line: Some(3),
                column: Some(1),
            },
        });
        let msg = err.to_string();
        assert!(msg.contains("bad.yaml"));
        assert!(msg.contains("unexpected key"));
    }

    #[test]
    fn error_spec_display() {
        let err = Error::Spec(SpecError {
            path: std::path::PathBuf::from("commands.yaml"),
            details: FileParseError {
                format: "YAML",
                message: "invalid nargs".into(),
                line: None,
                column: None,
            },
        });
        let msg = err.to_string();
        assert!(msg.contains("commands.yaml"));
        assert!(msg.contains("invalid nargs"));
    }

    #[test]
    fn error_io_display() {
        let err = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn error_formatter_display() {
        let err = Error::Formatter("something went wrong".to_owned());
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn error_layout_too_wide_display() {
        let err = Error::LayoutTooWide {
            line_no: 42,
            width: 120,
            limit: 80,
        };
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("120"));
        assert!(msg.contains("80"));
    }

    #[test]
    fn with_display_name_updates_parse() {
        let err = Error::Parse(ParseError {
            display_name: "original".to_owned(),
            source_text: "set(\n".into(),
            start_line: 1,
            diagnostic: ParseDiagnostic {
                message: "test".into(),
                line: 1,
                column: 5,
            },
        });
        let renamed = err.with_display_name("renamed.cmake");
        match renamed {
            Error::Parse(ParseError { display_name, .. }) => {
                assert_eq!(display_name, "renamed.cmake");
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn with_display_name_passes_through_non_parse_errors() {
        let err = Error::Formatter("test".to_owned());
        let result = err.with_display_name("ignored");
        match result {
            Error::Formatter(msg) => assert_eq!(msg, "test"),
            _ => panic!("expected Formatter to pass through"),
        }
    }

    #[test]
    fn io_error_converts_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: Error = io_err.into();
        match err {
            Error::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
            _ => panic!("expected Io variant"),
        }
    }
}
