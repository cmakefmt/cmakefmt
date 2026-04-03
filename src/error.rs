use thiserror::Error;

/// Errors that can be returned by parsing, config loading, spec loading, or
/// formatting operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A syntax error reported by the CMake parser.
    #[error("parse error: {0}")]
    Parse(#[from] Box<pest::error::Error<crate::parser::Rule>>),

    /// A `.cmake-format.toml` parse error.
    #[error("config error in {path}: {source}")]
    Config {
        /// The config file that failed to deserialize.
        path: std::path::PathBuf,
        /// The underlying TOML deserialization error.
        source: toml::de::Error,
    },

    /// A built-in or user override spec parse error.
    #[error("spec error in {path}: {source}")]
    Spec {
        /// The spec file that failed to deserialize.
        path: std::path::PathBuf,
        /// The underlying TOML deserialization error.
        source: toml::de::Error,
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
