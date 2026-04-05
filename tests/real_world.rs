// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use cmakefmt::{format_source, Config};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RealWorldManifest {
    fixture: Vec<RealWorldFixture>,
}

#[derive(Debug, Deserialize)]
struct RealWorldFixture {
    name: String,
    relative_path: String,
    source_url: String,
    raw_url: String,
    sha256: String,
}

fn load_manifest() -> RealWorldManifest {
    toml::from_str(&fs::read_to_string("tests/fixtures/real_world/manifest.toml").unwrap()).unwrap()
}

fn real_world_corpus_root() -> PathBuf {
    env::var_os("CMAKEFMT_REAL_WORLD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/real-world-corpus"))
}

fn available_real_world_fixture_paths() -> Option<Vec<PathBuf>> {
    let manifest = load_manifest();
    let root = real_world_corpus_root();

    if !root.exists() {
        eprintln!(
            "skipping real-world corpus tests: {} does not exist; run `python3 scripts/fetch-real-world-corpus.py`",
            root.display()
        );
        return None;
    }

    let mut paths = Vec::with_capacity(manifest.fixture.len());
    for fixture in &manifest.fixture {
        let path = root.join(&fixture.relative_path);
        if !path.is_file() {
            eprintln!(
                "skipping real-world corpus tests: missing {}; run `python3 scripts/fetch-real-world-corpus.py`",
                path.display()
            );
            return None;
        }
        paths.push(path);
    }

    Some(paths)
}

#[test]
fn real_world_manifest_has_expected_size() {
    let fixtures = load_manifest().fixture;
    assert!(
        fixtures.len() >= 10,
        "expected at least 10 real-world manifest entries, found {}",
        fixtures.len()
    );
}

#[test]
fn real_world_manifest_entries_are_unique_and_complete() {
    let manifest = load_manifest();
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();

    for fixture in &manifest.fixture {
        assert!(
            names.insert(fixture.name.as_str()),
            "duplicate fixture name {}",
            fixture.name
        );
        assert!(
            paths.insert(fixture.relative_path.as_str()),
            "duplicate fixture path {}",
            fixture.relative_path
        );
        assert!(
            fixture.source_url.starts_with("https://"),
            "source_url must be https for {}",
            fixture.name
        );
        assert!(
            fixture.raw_url.starts_with("https://"),
            "raw_url must be https for {}",
            fixture.name
        );
        assert_eq!(
            fixture.sha256.len(),
            64,
            "sha256 must be 64 hex characters for {}",
            fixture.name
        );
    }
}

#[test]
fn real_world_outputs_are_idempotent() {
    let Some(paths) = available_real_world_fixture_paths() else {
        return;
    };

    let config = Config::default();

    for path in paths {
        let source = fs::read_to_string(&path).unwrap();
        let formatted = format_source(&source, &config)
            .unwrap_or_else(|err| panic!("formatting {} failed: {err}", path.display()));
        let reformatted = format_source(&formatted, &config)
            .unwrap_or_else(|err| panic!("re-formatting {} failed: {err}", path.display()));
        assert_eq!(
            formatted,
            reformatted,
            "formatted output for {} was not idempotent",
            path.display()
        );
    }
}
