// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    pre_serialize_builtins_spec();
    pre_serialize_modules_spec();

    // Skip git version embedding when cross-compiling for WASM.
    if env::var("TARGET").is_ok_and(|t| t.contains("wasm32")) {
        return;
    }

    println!("cargo:rerun-if-env-changed=CMAKEFMT_BUILD_GIT_SHA");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_owned());
    let long_version = match git_sha() {
        Some(sha) => format!("{version} ({sha})"),
        None => version,
    };

    println!("cargo:rustc-env=CMAKEFMT_CLI_LONG_VERSION={long_version}");
}

/// Parse `src/spec/builtins.yaml` once at build time and re-emit it as
/// MessagePack into `${OUT_DIR}/builtins.msgpack`. The runtime registry
/// loads that blob via `rmp-serde`, which is ~20× faster than parsing
/// YAML at every process startup.
///
/// We deliberately serialise through an opaque `serde_yaml::Value` so the
/// build script does not need to know the real `SpecFile` schema —
/// MessagePack is self-describing and `rmp-serde` happily roundtrips
/// the generic value tree into the typed structs at runtime, including
/// the `#[serde(untagged)]` `CommandSpec` enum which schema-based
/// formats (e.g. `bincode`/`postcard`) cannot handle.
fn pre_serialize_builtins_spec() {
    pre_serialize_yaml_spec("src/spec/builtins.yaml", "builtins.msgpack");
}

/// Same dance for `src/spec/modules.yaml`, the spec file covering
/// commands defined in CMake's bundled modules (FetchContent,
/// ExternalProject, GoogleTest, the Check* family, etc.) rather than
/// the language builtins. Loaded alongside `builtins.msgpack` and
/// merged at startup so the runtime sees a single command table.
fn pre_serialize_modules_spec() {
    pre_serialize_yaml_spec("src/spec/modules.yaml", "modules.msgpack");
}

fn pre_serialize_yaml_spec(yaml_path: &str, msgpack_filename: &str) {
    println!("cargo:rerun-if-changed={yaml_path}");
    let yaml =
        std::fs::read_to_string(yaml_path).unwrap_or_else(|e| panic!("read {yaml_path}: {e}"));
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml)
        .unwrap_or_else(|e| panic!("parse {yaml_path} as serde_yaml::Value: {e}"));
    let bytes = rmp_serde::to_vec(&value)
        .unwrap_or_else(|e| panic!("serialise {yaml_path} as MessagePack: {e}"));
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let out_path = PathBuf::from(out_dir).join(msgpack_filename);
    std::fs::write(&out_path, bytes)
        .unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));
}

fn git_sha() -> Option<String> {
    if let Ok(explicit) = env::var("CMAKEFMT_BUILD_GIT_SHA") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim();
    (!sha.is_empty()).then(|| sha.to_owned())
}
