// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! WebAssembly entry points for the browser playground.

use wasm_bindgen::prelude::*;

use crate::config::Config;
use crate::spec::registry::CommandRegistry;

/// Format CMake source code with the given YAML config.
///
/// The YAML can include a `commands:` section defining custom command specs
/// alongside the normal formatter config fields. Unknown fields (like
/// `commands`) are ignored when parsing the formatter config.
///
/// Returns the formatted source string, or throws a JS error on failure.
#[wasm_bindgen]
pub fn format(source: &str, config_yaml: &str) -> Result<String, JsValue> {
    // Parse formatter config (ignores unknown keys like `commands`).
    let config: Config = serde_yaml::from_str(config_yaml)
        .map_err(|e| JsValue::from_str(&format!("config error: {e}")))?;

    // Use load() (not builtins()) to get a fresh owned copy each time,
    // since the user may change the commands spec between format calls.
    let mut registry =
        CommandRegistry::load().map_err(|e| JsValue::from_str(&format!("registry error: {e}")))?;

    // Extract the `commands` section if present and merge as spec overrides.
    if let Ok(mapping) = serde_yaml::from_str::<serde_yaml::Mapping>(config_yaml) {
        let key = serde_yaml::Value::String("commands".into());
        if let Some(commands_val) = mapping.get(&key) {
            if !commands_val.is_null() {
                let mut wrapper = serde_yaml::Mapping::new();
                wrapper.insert(key, commands_val.clone());
                let yaml_str = serde_yaml::to_string(&wrapper)
                    .map_err(|e| JsValue::from_str(&format!("spec error: {e}")))?;
                registry
                    .merge_yaml_overrides(&yaml_str)
                    .map_err(|e| JsValue::from_str(&format!("spec error: {e}")))?;
            }
        }
    }

    crate::format_source_with_registry(source, &config, &registry)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Return the default configuration as a pretty-printed JSON string.
#[wasm_bindgen]
pub fn default_config_json() -> String {
    serde_json::to_string_pretty(&Config::default()).unwrap()
}
