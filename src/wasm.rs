// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! WebAssembly entry points for the browser playground.

use wasm_bindgen::prelude::*;

use crate::config::Config;
use crate::spec::registry::CommandRegistry;

/// Format CMake source code with the given YAML config.
///
/// The YAML uses the same schema as `.cmakefmt.yaml` files, with `format:`,
/// `markup:`, and optional `commands:` sections. Unknown fields
/// are rejected.
///
/// Returns the formatted source string, or throws a JS error on failure.
#[wasm_bindgen]
pub fn format(source: &str, config_yaml: &str) -> Result<String, JsValue> {
    let (config, commands) = Config::from_yaml_str(config_yaml)
        .map_err(|e| JsValue::from_str(&format!("config error: {e}")))?;

    let mut registry =
        CommandRegistry::load().map_err(|e| JsValue::from_str(&format!("registry error: {e}")))?;

    if let Some(commands_val) = commands {
        if !commands_val.is_null() {
            let key = serde_yaml::Value::String("commands".into());
            let mut wrapper = serde_yaml::Mapping::new();
            wrapper.insert(key, commands_val);
            let yaml_str = serde_yaml::to_string(&wrapper)
                .map_err(|e| JsValue::from_str(&format!("spec error: {e}")))?;
            registry
                .merge_yaml_overrides(&yaml_str)
                .map_err(|e| JsValue::from_str(&format!("spec error: {e}")))?;
        }
    }

    crate::format_source_with_registry(source, &config, &registry)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Return the default configuration as a YAML string.
///
/// The returned string uses the same sectioned schema (`format:`,
/// `markup:`) as `.cmakefmt.yaml` files and can be passed
/// directly to `format()`.
#[wasm_bindgen]
pub fn default_config_yaml() -> String {
    crate::default_config_template()
}
