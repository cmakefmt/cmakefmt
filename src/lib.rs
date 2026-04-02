pub mod config;
pub mod error;
pub mod files;
pub mod formatter;
pub mod parser;
pub mod spec;

pub use config::{
    default_config_template, CaseStyle, CommandConfig, Config, DangleAlign, PerCommandConfig,
};
pub use error::{Error, Result};
pub use formatter::{
    format_source, format_source_with_debug, format_source_with_registry,
    format_source_with_registry_debug,
};
