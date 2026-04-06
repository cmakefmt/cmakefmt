// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::env;
use std::process::Command;

fn main() {
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
