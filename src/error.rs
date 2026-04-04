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

/// Errors that can be returned by parsing, config loading, spec loading, or
/// formatting operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A syntax error reported by the CMake parser.
    #[error("parse error: {0}")]
    Parse(#[from] Box<pest::error::Error<crate::parser::Rule>>),

    /// A parser error annotated with source text and line-offset context.
    #[error("parse error in {display_name}: {source}")]
    ParseContext {
        /// Human-facing source name, for example a path or `<stdin>`.
        display_name: String,
        /// The source text that failed to parse.
        source_text: Box<str>,
        /// The 1-based source line number where this parser chunk started.
        start_line: usize,
        /// Whether earlier barrier/fence handling affected how this chunk was parsed.
        barrier_context: bool,
        /// The underlying pest parser error.
        source: Box<pest::error::Error<crate::parser::Rule>>,
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
                source,
                ..
            } => Self::ParseContext {
                display_name: display_name.into(),
                source_text,
                start_line,
                barrier_context,
                source,
            },
            other => other,
        }
    }
}
