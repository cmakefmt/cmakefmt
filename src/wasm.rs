// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! WebAssembly entry points for the browser playground.

use wasm_bindgen::prelude::*;

use crate::config::Config;

/// Format CMake source code with the given JSON-encoded configuration.
///
/// Returns the formatted source string, or throws a JS error on failure.
#[wasm_bindgen]
pub fn format(source: &str, config_json: &str) -> Result<String, JsValue> {
    let config: Config = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("config error: {e}")))?;
    crate::format_source(source, &config).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Return the default configuration as a pretty-printed JSON string.
#[wasm_bindgen]
pub fn default_config_json() -> String {
    serde_json::to_string_pretty(&Config::default()).unwrap()
}
