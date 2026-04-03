use std::fs;
use std::path::{Path, PathBuf};

use cmakefmt::{format_source, Config};
use walkdir::WalkDir;

fn real_world_fixture_paths() -> Vec<PathBuf> {
    let root = Path::new("tests/fixtures/real_world");
    let mut paths: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("CMakeLists.txt"))
        .filter(|path| path.parent().is_some_and(|parent| parent != root))
        .collect();
    paths.sort();
    paths
}

fn snapshot_name(path: &Path) -> String {
    let relative = path
        .strip_prefix("tests/fixtures/real_world")
        .unwrap()
        .to_string_lossy();
    relative
        .replace(std::path::MAIN_SEPARATOR, "__")
        .replace('.', "_")
}

#[test]
fn real_world_corpus_has_expected_size() {
    let fixtures = real_world_fixture_paths();
    assert!(
        fixtures.len() >= 10,
        "expected at least 10 real-world fixtures, found {}",
        fixtures.len()
    );
}

#[test]
fn real_world_fixture_manifest_mentions_every_fixture() {
    let manifest = fs::read_to_string("tests/fixtures/real_world/SOURCES.md").unwrap();

    for path in real_world_fixture_paths() {
        let relative = path
            .strip_prefix("tests/fixtures/real_world")
            .unwrap()
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        assert!(
            manifest.contains(&relative),
            "fixture {} missing from SOURCES.md",
            relative
        );
    }
}

#[test]
fn real_world_outputs_match_snapshots() {
    let config = Config::default();

    for path in real_world_fixture_paths() {
        let source = fs::read_to_string(&path).unwrap();
        let formatted = format_source(&source, &config)
            .unwrap_or_else(|err| panic!("formatting {} failed: {err}", path.display()));
        insta::assert_snapshot!(snapshot_name(&path), formatted);
    }
}
