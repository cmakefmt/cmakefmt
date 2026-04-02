pub mod config;
pub mod error;
pub mod formatter;
pub mod parser;
pub mod spec;

pub use config::{
    default_config_template, CaseStyle, CommandConfig, Config, DangleAlign, PerCommandConfig,
};
pub use error::{Error, Result};
pub use formatter::format_source;
