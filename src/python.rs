// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Python bindings for cmakefmt via PyO3.

use pyo3::exceptions::PyOSError;
use pyo3::prelude::*;

use crate::config::Config;
use crate::spec::registry::CommandRegistry;

// ── Custom exception types ──────────────────────────────────────────────────

pyo3::create_exception!(cmakefmt, ParseError, pyo3::exceptions::PyException);
pyo3::create_exception!(cmakefmt, ConfigError, pyo3::exceptions::PyException);
pyo3::create_exception!(cmakefmt, FormatterError, pyo3::exceptions::PyException);
pyo3::create_exception!(cmakefmt, LayoutError, pyo3::exceptions::PyException);

fn convert_error(err: crate::Error) -> PyErr {
    match err {
        crate::Error::ParseContext { .. } => ParseError::new_err(err.to_string()),
        crate::Error::Config { .. } | crate::Error::Spec { .. } => {
            ConfigError::new_err(err.to_string())
        }
        crate::Error::Formatter(msg) => FormatterError::new_err(msg),
        crate::Error::LayoutTooWide {
            line_no,
            width,
            limit,
        } => LayoutError::new_err(format!(
            "line {line_no} is {width} chars, exceeding limit of {limit}"
        )),
        crate::Error::Io(e) => PyOSError::new_err(e.to_string()),
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Format CMake source code.
///
/// Args:
///     source: CMake source code as a string.
///     config: Optional YAML config string (same format as .cmakefmt.yaml).
///         Supports ``format:``, ``markup:``, ``per_command_overrides:``,
///         and ``commands:`` sections.
///
/// Returns:
///     Formatted source code as a string.
///
/// Raises:
///     ParseError: If the source cannot be parsed as CMake.
///     ConfigError: If the config is invalid.
///     FormatterError: If formatting fails.
///     LayoutError: If require_valid_layout is true and a line exceeds line_width.
#[pyfunction]
#[pyo3(signature = (source, *, config=None))]
fn format_source(source: &str, config: Option<&str>) -> PyResult<String> {
    let (config, commands_value) = resolve_config(config)?;
    let registry = build_registry(commands_value)?;

    crate::format_source_with_registry(source, &config, &registry).map_err(convert_error)
}

/// Check if source is already correctly formatted.
///
/// Args:
///     source: CMake source code as a string.
///     config: Optional YAML config string.
///
/// Returns:
///     True if the source is already formatted, False if it would change.
#[pyfunction]
#[pyo3(signature = (source, *, config=None))]
fn is_formatted(source: &str, config: Option<&str>) -> PyResult<bool> {
    let formatted = format_source(source, config)?;
    Ok(formatted == source)
}

/// Return the default configuration as a YAML string.
///
/// The returned string uses the same format as ``.cmakefmt.yaml`` files
/// and can be passed directly to ``format_source(config=...)``.
#[pyfunction]
fn default_config() -> PyResult<String> {
    Ok(crate::default_config_template())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn resolve_config(config: Option<&str>) -> PyResult<(Config, Option<Box<str>>)> {
    if let Some(yaml) = config {
        Config::from_yaml_str_with_commands(yaml).map_err(convert_error)
    } else {
        Ok((Config::default(), None))
    }
}

fn build_registry(commands_yaml: Option<Box<str>>) -> PyResult<CommandRegistry> {
    let mut registry = CommandRegistry::load().map_err(convert_error)?;
    if let Some(commands_yaml) = commands_yaml {
        registry
            .merge_yaml_overrides(commands_yaml.as_ref())
            .map_err(convert_error)?;
    }
    Ok(registry)
}

// ── Module definition ───────────────────────────────────────────────────────

#[pymodule]
fn _cmakefmt(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(format_source, m)?)?;
    m.add_function(wrap_pyfunction!(is_formatted, m)?)?;
    m.add_function(wrap_pyfunction!(default_config, m)?)?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("ConfigError", m.py().get_type::<ConfigError>())?;
    m.add("FormatterError", m.py().get_type::<FormatterError>())?;
    m.add("LayoutError", m.py().get_type::<LayoutError>())?;
    Ok(())
}
