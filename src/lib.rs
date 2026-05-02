// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg_attr(docsrs, feature(doc_cfg))]

//! `cmakefmt` is a fast, configurable CMake formatter.
//!
//! # Quick start
//!
//! Format a CMake source string with the default configuration:
//!
//! ```
//! use cmakefmt::{format_source, Config};
//!
//! let cmake = "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n";
//! let formatted = format_source(cmake, &Config::default()).unwrap();
//! assert_eq!(formatted, "cmake_minimum_required(VERSION 3.20)\n");
//! ```
//!
//! To customise formatting, modify [`Config`] fields before passing it in:
//!
//! ```
//! use cmakefmt::{format_source, Config, CaseStyle};
//!
//! let mut config = Config::default();
//! config.line_width = 100;
//! config.command_case = CaseStyle::Upper;
//!
//! let cmake = "target_link_libraries(mylib PUBLIC dep1)\n";
//! let formatted = format_source(cmake, &config).unwrap();
//! assert_eq!(formatted, "TARGET_LINK_LIBRARIES(mylib PUBLIC dep1)\n");
//! ```
//!
//! # Organisation
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`config`] | Runtime configuration types and config-file loading |
//! | [`error`] | Error and result types |
//! | [`formatter`] | Source-to-source formatting pipeline |
//! | [`parser`] | CMake parser and AST definitions |
//! | [`spec`] | Built-in and user-extensible command registry |
//!
//! # Entry points (re-exported at the crate root)
//!
//! | Item | Purpose |
//! |------|---------|
//! | [`format_source`] / [`format_source_with_registry`] | Format a source string |
//! | [`format_parsed_file`] | Format an already-parsed [`parser::ast::File`] |
//! | [`format_source_with_debug`] / [`format_source_with_registry_debug`] | Format + collect debug decision log |
//! | [`Config`], [`CommandConfig`], [`PerCommandConfig`] | Runtime configuration |
//! | [`CaseStyle`], [`DangleAlign`], [`LineEnding`], [`FractionalTabPolicy`] | Config enums |
//! | [`CommandRegistry`] | Built-in + user-override command specs |
//! | [`Error`], [`Result`], [`IoResultExt`] | Crate-level error types and the path-context adapter for `io::Result` |
//!
//! # Features
//!
//! | Feature | Default | Purpose |
//! |---------|---------|---------|
//! | `cli` | ✔ | Enables the `cmakefmt` binary plus CLI-oriented public API (`convert_legacy_config_files`, `default_config_template_for`, `generate_json_schema`, `render_effective_config`, `DumpConfigFormat`). Implies `lsp`. |
//! | `lsp` | ✔ (via `cli`) | Compiles the `lsp::run` Language Server Protocol entry point. |
//!
//! The crate also has a separate target path: when compiled for
//! `wasm32`, `wasm::format` and friends are exposed via
//! `wasm-bindgen` for the browser playground.

/// Runtime formatter configuration and config-file loading.
pub mod config;
/// Shared error types used across parsing, config loading, and formatting.
pub mod error;
/// Source-to-source formatting pipeline.
pub mod formatter;
/// CMake parser and AST definitions.
pub mod parser;
/// Semantic-level normalisation (strip comments, line endings,
/// keyword casing) used by `--verify` and the idempotency tests.
pub mod semantic;
/// Built-in and user-extensible command specification registry.
pub mod spec;

// Recursive CMake file-discovery helpers used by the CLI.  Not part of the
// library embedding API; hidden from generated documentation.
#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod files;

// AST / parse-tree dumping helpers used by the `dump` subcommand.
#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod dump;

// LSP server — only compiled when the `lsp` feature is enabled.
#[cfg(feature = "lsp")]
#[cfg_attr(docsrs, doc(cfg(feature = "lsp")))]
pub mod lsp;

// WASM entry point — only compiled for wasm32 targets.
#[cfg(target_arch = "wasm32")]
#[cfg_attr(docsrs, doc(cfg(target_arch = "wasm32")))]
pub mod wasm;

// ── Configuration ────────────────────────────────────────────────────────────

pub use config::{
    CaseStyle, CommandConfig, Config, ContinuationAlign, DangleAlign, FractionalTabPolicy,
    LineEnding, PerCommandConfig,
};

pub use config::default_config_template;

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub use config::{
    convert_legacy_config_files, default_config_template_for, generate_json_schema,
    render_effective_config, DumpConfigFormat,
};

// ── Errors ───────────────────────────────────────────────────────────────────

pub use error::{Error, IoResultExt, Result};

// ── Formatting ───────────────────────────────────────────────────────────────

pub use formatter::{
    format_parsed_file, format_source, format_source_with_debug, format_source_with_registry,
    format_source_with_registry_debug,
};

// ── Registry ─────────────────────────────────────────────────────────────────

pub use spec::registry::CommandRegistry;
