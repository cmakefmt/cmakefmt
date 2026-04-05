// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cmakefmt` is a fast, configurable CMake formatter and parser.
//!
//! The crate exposes:
//!
//! - configuration types under [`config`]
//! - parser entry points under [`parser`]
//! - formatting entry points under [`formatter`]
//! - command-spec registry types under [`spec`]
//!
//! Most embedders will start with [`format_source`] or
//! [`format_source_with_debug`].

/// Runtime formatter configuration and config-file loading.
pub mod config;
/// Shared error types used across parsing, config loading, and formatting.
pub mod error;
/// Recursive CMake file discovery helpers used by the CLI and benchmarks.
pub mod files;
/// Source-to-source formatting pipeline.
pub mod formatter;
/// CMake parser and AST definitions.
pub mod parser;
/// Built-in and user-extensible command specification registry.
pub mod spec;

/// Re-exported configuration entry points.
pub use config::{
    convert_legacy_config_files, default_config_template, default_config_template_for,
    render_effective_config, CaseStyle, CommandConfig, Config, DangleAlign, DumpConfigFormat,
    PerCommandConfig,
};
/// Re-exported error types.
pub use error::{Error, Result};
/// Re-exported formatter entry points.
pub use formatter::{
    format_source, format_source_with_debug, format_source_with_registry,
    format_source_with_registry_debug,
};
