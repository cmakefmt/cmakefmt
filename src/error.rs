use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error: {0}")]
    Parse(#[from] Box<pest::error::Error<crate::parser::Rule>>),

    #[error("config error in {path}: {source}")]
    Config {
        path: std::path::PathBuf,
        source: toml::de::Error,
    },

    #[error("spec error in {path}: {source}")]
    Spec {
        path: std::path::PathBuf,
        source: toml::de::Error,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("formatter error: {0}")]
    Formatter(String),
}

pub type Result<T> = std::result::Result<T, Error>;
