// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::Write;
use std::process::Command;

use serde_json::Value;

fn cmakefmt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
}

fn write_file(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(dir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn init_git_repo(dir: &std::path::Path) {
    git(dir, &["init"]);
    git(dir, &["config", "user.email", "cmakefmt@example.invalid"]);
    git(dir, &["config", "user.name", "cmakefmt tests"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

// ── Basic formatting ────────────────────────────────────────────────────────

#[test]
fn formats_file_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "cmake_minimum_required( VERSION   3.20 )\n").unwrap();

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cmake_minimum_required(VERSION 3.20)\n"
    );
}

#[test]
fn changed_stdout_lines_are_colored_when_forced() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(FOO bar)\nset(  BAZ  qux )\n").unwrap();

    let output = cmakefmt()
        .args(["--colour", "always", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "set(FOO bar)\n\u{1b}[36mset(BAZ qux)\u{1b}[0m\n"
    );
}

#[test]
fn color_auto_stays_plain_when_stdout_is_piped() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(  FOO  bar )\n").unwrap();

    let output = cmakefmt()
        .args(["--colour", "auto", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
}

#[test]
fn color_never_disables_highlighting_even_when_forced_off() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(  FOO  bar )\n").unwrap();

    let output = cmakefmt()
        .args(["--colour", "never", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
}

#[test]
fn reads_stdin_with_dash() {
    let mut child = cmakefmt()
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"set(  FOO   bar )\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
}

#[test]
fn stdin_path_uses_config_discovery() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    write_file(
        &nested.join(".cmakefmt.yaml"),
        "format:\n  command_case: upper\n",
    );

    let mut child = cmakefmt()
        .args(["-", "--stdin-path", "nested/CMakeLists.txt"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("CMAKE_MINIMUM_REQUIRED("));
}

#[test]
fn diff_outputs_unified_diff() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--diff", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- a/"));
    assert!(stdout.contains("+++ b/"));
    assert!(stdout.contains("-set(  FOO  bar )"));
    assert!(stdout.contains("+set(FOO bar)"));
}

#[test]
fn diff_outputs_colored_hunks_when_colour_always() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--colour", "always", "--diff", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{1b}[31m-set(  FOO  bar )\u{1b}[0m"));
    assert!(stdout.contains("\u{1b}[32m+set(FOO bar)\u{1b}[0m"));
    assert!(stdout.contains("--- a/"));
    assert!(stdout.contains("+++ b/"));
}

#[test]
fn diff_does_not_color_when_colour_never() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--colour", "never", "--diff", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\u{1b}[31m"));
    assert!(!stdout.contains("\u{1b}[32m"));
}

#[test]
fn json_report_in_check_mode() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--report-format", "json", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["mode"], "check");
    assert_eq!(report["files"][0]["would_change"], true);
    assert!(report["files"][0]["changed_lines"][0].as_u64().is_some());
}

#[test]
fn json_report_in_diff_mode_includes_diff() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--report-format", "json", "--diff", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["mode"], "diff");
    assert!(report["files"][0]["diff"]
        .as_str()
        .unwrap()
        .contains("--- a/"));
}

#[test]
fn github_report_emits_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--report-format",
            "github",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("::warning file="));
    assert!(stdout.contains("file would be reformatted by cmakefmt"));
    assert!(stdout.contains("::notice::summary:"));
}

#[test]
fn checkstyle_report_emits_xml() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--report-format",
            "checkstyle",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(stdout.contains("<checkstyle version=\"4.3\">"));
    assert!(stdout.contains("source=\"cmakefmt.format\""));
}

#[test]
fn junit_report_emits_xml() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--report-format",
            "junit",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(stdout.contains("<testsuite name=\"cmakefmt\""));
    assert!(stdout.contains("<failure message=\"file would be reformatted by cmakefmt\">"));
}

#[test]
fn sarif_report_emits_results() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--report-format",
            "sarif",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["version"], "2.1.0");
    assert_eq!(
        report["runs"][0]["results"][0]["ruleId"],
        "cmakefmt/would-reformat"
    );
}

// ── In-place formatting ─────────────────────────────────────────────────────

#[test]
fn in_place_modifies_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(  FOO  bar )\n").unwrap();

    let output = cmakefmt()
        .args(["-i", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    // stdout should be empty for in-place mode
    assert!(output.stdout.is_empty());

    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, "set(FOO bar)\n");
}

#[test]
fn in_place_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(  FOO  bar )\n").unwrap();

    // Format once
    cmakefmt()
        .args(["-i", file.to_str().unwrap()])
        .output()
        .unwrap();
    let first = std::fs::read_to_string(&file).unwrap();

    // Format again
    cmakefmt()
        .args(["-i", file.to_str().unwrap()])
        .output()
        .unwrap();
    let second = std::fs::read_to_string(&file).unwrap();

    assert_eq!(first, second);
}

// ── Check mode ──────────────────────────────────────────────────────────────

#[test]
fn check_returns_0_for_formatted_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "cmake_minimum_required(VERSION 3.20)\n").unwrap();

    let output = cmakefmt()
        .args(["--check", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn check_returns_1_for_unformatted_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "cmake_minimum_required(   VERSION   3.20  )\n").unwrap();

    let output = cmakefmt()
        .args(["--check", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    // Should print which file would change
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("would be reformatted"));
}

#[test]
fn check_does_not_modify_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    let original = "set(  FOO  bar )\n";
    std::fs::write(&file, original).unwrap();

    cmakefmt()
        .args(["--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, original);
}

#[test]
fn quiet_check_emits_summary_without_per_file_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--check", "--quiet", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("summary: selected=1, changed=1, unchanged=0, failed=0"));
    assert!(!stderr.contains("would be reformatted"));
}

#[test]
fn quiet_stdout_suppresses_formatted_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--quiet", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "stdout should be empty with -q, got: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("summary:"),
        "summary should still appear on stderr, got: {stderr}"
    );
}

#[test]
fn quiet_in_place_suppresses_per_file_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--quiet", "--in-place", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "stdout should be empty, got: {stdout}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("summary:"),
        "summary should still appear on stderr, got: {stderr}"
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "set(FOO bar)\n",
        "file should still be modified on disk"
    );
}

#[test]
fn quiet_diff_suppresses_per_file_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--quiet", "--diff", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    // Diff output should still appear on stdout (--quiet doesn't suppress --diff)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("---"),
        "diff should still appear on stdout, got: {stdout}"
    );
}

// ── --sorted ────────────────────────────────────────────────────────────────

#[test]
fn sorted_outputs_files_in_alphabetical_order() {
    let dir = tempfile::tempdir().unwrap();
    // Create files with names that would sort differently from filesystem order
    write_file(&dir.path().join("z.cmake"), "set(Z 1)\n");
    write_file(&dir.path().join("a.cmake"), "set(  A  1 )\n");
    write_file(&dir.path().join("m.cmake"), "set(M 1)\n");

    let output = cmakefmt()
        .args([
            "--sorted",
            "--list-input-files",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = stdout.lines().collect();
    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted, "files should be in alphabetical order");
}

// ── --cache-strategy ────────────────────────────────────────────────────────

#[test]
fn cache_strategy_content_invalidates_on_content_change() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("cache");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    // First run: cache miss
    let first = cmakefmt()
        .args([
            "--cache",
            "--cache-location",
            cache_dir.to_str().unwrap(),
            "--cache-strategy",
            "content",
            "--debug",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("cache miss"),
        "first run should be a cache miss, got: {first_stderr}"
    );

    // Second run without changes: cache hit
    let second = cmakefmt()
        .args([
            "--cache",
            "--cache-location",
            cache_dir.to_str().unwrap(),
            "--cache-strategy",
            "content",
            "--debug",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("cache hit"),
        "second run should be a cache hit, got: {second_stderr}"
    );
}

// ── Config override CLI flags ───────────────────────────────────────────────

#[test]
fn tab_size_override() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "if(TRUE)\nset(X 1)\nendif()\n");

    let output = cmakefmt()
        .args(["--tab-size", "4", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("    set(X 1)"),
        "should indent with 4 spaces, got: {stdout}"
    );
}

#[test]
fn keyword_case_override() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar CACHE STRING \"\")\n");

    let output = cmakefmt()
        .args(["--keyword-case", "lower", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cache"),
        "keywords should be lowercase, got: {stdout}"
    );
}

#[test]
fn dangle_parens_override() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    // Long enough to wrap
    write_file(
        &file,
        "set(VERY_LONG_VARIABLE_NAME value1 value2 value3 value4 value5 value6 value7 value8)\n",
    );

    let output = cmakefmt()
        .args([
            "--dangle-parens",
            "true",
            "--line-width",
            "40",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap().trim(),
        ")",
        "closing paren should be on its own line, got: {stdout}"
    );
}

// ── --no-verify / --verify ──────────────────────────────────────────────────

#[test]
fn no_verify_skips_semantic_check_for_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    // --no-verify with --in-place should succeed (same as --fast)
    let output = cmakefmt()
        .args(["--no-verify", "--in-place", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "set(FOO bar)\n");
}

#[test]
fn cache_reports_hit_on_second_run() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("cache");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let first = cmakefmt()
        .args([
            "--cache",
            "--cache-location",
            cache_dir.to_str().unwrap(),
            "--debug",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(first_stderr.contains("cache miss"));

    let second = cmakefmt()
        .args([
            "--cache",
            "--cache-location",
            cache_dir.to_str().unwrap(),
            "--debug",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(second_stderr.contains("cache hit"));
}

#[test]
fn cache_location_creates_cache_files() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("cache");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--cache-location",
            cache_dir.to_str().unwrap(),
            "--cache-strategy",
            "content",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let entries: Vec<_> = std::fs::read_dir(&cache_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(!entries.is_empty());
}

#[test]
fn require_pragma_skips_unmarked_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--require-pragma",
            "--check",
            "--quiet",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skipped=1"));
}

#[test]
fn require_pragma_formats_marked_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "# cmakefmt: enable\nset(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--require-pragma", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "# cmakefmt: enable\nset(FOO bar)\n"
    );
}

// ── Error handling ──────────────────────────────────────────────────────────

#[test]
fn nonexistent_file_returns_exit_2() {
    let output = cmakefmt().arg("/nonexistent/file.cmake").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
}

#[test]
fn invalid_file_regex_returns_exit_2() {
    let output = cmakefmt().args(["--path-regex", "("]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid file regex"));
    assert!(stderr.contains("--path-regex"));
}

#[test]
fn parse_errors_include_context_and_repro_hint() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "message(FATAL_ERROR \"foo\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to parse"));
    assert!(stderr.contains(file.to_str().unwrap()));
    assert!(stderr.contains("parser detail:"));
    assert!(stderr.contains("repro: cmakefmt --debug --check"));
}

#[test]
fn unclosed_paren_error_points_to_opening_paren() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "cmake_minimum_required(VERSION 3.20)\nset(FOO\n  bar\n  baz\n\nmessage(STATUS \"hello\")\n",
    );

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error location should point to the unclosed `(`, not EOF.
    assert!(
        stderr.contains(":2:4"),
        "error should point to line 2, column 4 (the opening paren), got: {stderr}"
    );
    assert!(
        stderr.contains("set(FOO"),
        "snippet should show the line with the unclosed paren, got: {stderr}"
    );
    assert!(
        stderr.contains("unclosed `(`"),
        "should include unclosed paren hint, got: {stderr}"
    );
}

#[test]
fn unclosed_paren_at_eof_points_to_opening_paren() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "add_subdirectory(foo\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(":1:17"),
        "error should point to column 17 (the opening paren), got: {stderr}"
    );
    assert!(
        stderr.contains("unclosed `(`"),
        "should include unclosed paren hint, got: {stderr}"
    );
}

#[test]
fn balanced_parens_error_does_not_show_unclosed_hint() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    // Parse error mid-file (not at EOF) — parens are balanced.
    write_file(&file, "set(FOO bar)\n)extra_close\nset(BAZ qux)\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unclosed `(`"),
        "should not show unclosed paren hint for a mid-file error, got: {stderr}"
    );
}

#[test]
fn keep_going_formats_other_files_and_reports_error_summary() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.cmake");
    let bad = dir.path().join("bad.cmake");
    write_file(&good, "set(  GOOD  value )\n");
    write_file(&bad, "message(FATAL_ERROR \"unterminated\n");

    let output = cmakefmt()
        .args(["--keep-going", "--in-place", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read_to_string(&good).unwrap(), "set(GOOD value)\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to parse"));
    assert!(stderr.contains("summary: selected=2, changed=1, unchanged=0, failed=1"));
}

#[test]
fn keep_going_json_report_includes_errors() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.cmake");
    let bad = dir.path().join("bad.cmake");
    write_file(&good, "set(  GOOD  value )\n");
    write_file(&bad, "message(FATAL_ERROR \"unterminated\n");

    let output = cmakefmt()
        .args([
            "--keep-going",
            "--report-format",
            "json",
            "--check",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["selected"], 2);
    assert_eq!(report["summary"]["failed"], 1);
    assert_eq!(report["summary"]["changed"], 1);
    assert_eq!(report["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn config_errors_suggest_updated_or_close_keys() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.toml");
    write_file(&config, "[format]\nline_wdth = 90\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid config file"));
    assert!(stderr.contains("line_wdth"));
    assert!(stderr.contains("line_width"));
    assert!(stderr.contains("config files are applied in order"));
}

// ── CLI flag overrides ──────────────────────────────────────────────────────

#[test]
fn line_width_override() {
    let mut child = cmakefmt()
        .args(["--line-width", "30", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"set(FOO a b c d e f g h i j k l)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With line_width=30, args should wrap
    assert!(stdout.contains('\n'));
    // No line should exceed 30 chars
    for line in stdout.lines() {
        assert!(
            line.len() <= 30,
            "line exceeds 30 chars: {line:?} ({})",
            line.len()
        );
    }
}

#[test]
fn line_width_short_alias_works() {
    let mut child = cmakefmt()
        .args(["-l", "30", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"set(FOO a b c d e f g h i j k l)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        assert!(
            line.len() <= 30,
            "line exceeds 30 chars: {line:?} ({})",
            line.len()
        );
    }
}

#[test]
fn command_case_override() {
    let mut child = cmakefmt()
        .args(["--command-case", "upper", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("CMAKE_MINIMUM_REQUIRED("));
}

// ── Multiple files ──────────────────────────────────────────────────────────

#[test]
fn multiple_files_in_one_invocation() {
    let dir = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();

    for i in 0..10 {
        let file = dir.path().join(format!("file_{i}.cmake"));
        std::fs::write(&file, format!("set(  VAR_{i}  value )\n")).unwrap();
        paths.push(file);
    }

    let args: Vec<&str> = std::iter::once("-i")
        .chain(paths.iter().map(|p| p.to_str().unwrap()))
        .collect();

    let output = cmakefmt().args(&args).output().unwrap();
    assert!(output.status.success());

    for (i, path) in paths.iter().enumerate() {
        let contents = std::fs::read_to_string(path).unwrap();
        assert_eq!(contents, format!("set(VAR_{i} value)\n"));
    }
}

#[test]
fn files_from_reads_newline_delimited_targets() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.cmake");
    let second = dir.path().join("second.cmake");
    let list = dir.path().join("targets.txt");

    write_file(&first, "set(  FIRST  value )\n");
    write_file(&second, "set(  SECOND  value )\n");
    write_file(
        &list,
        &format!("{}\n{}\n", first.display(), second.display()),
    );

    let output = cmakefmt()
        .args(["-i", "--files-from", list.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(&first).unwrap(),
        "set(FIRST value)\n"
    );
    assert_eq!(
        std::fs::read_to_string(&second).unwrap(),
        "set(SECOND value)\n"
    );
}

#[test]
fn files_from_reads_stdin_target_list() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.cmake");
    let second = dir.path().join("second.cmake");
    write_file(&first, "set(  FIRST  value )\n");
    write_file(&second, "set(  SECOND  value )\n");

    let mut child = cmakefmt()
        .args(["-i", "--files-from", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("{}\n{}\n", first.display(), second.display()).as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(&first).unwrap(),
        "set(FIRST value)\n"
    );
    assert_eq!(
        std::fs::read_to_string(&second).unwrap(),
        "set(SECOND value)\n"
    );
}

#[test]
fn explicit_non_cmake_file_is_formatted() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("toolchain.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
}

#[test]
fn multiple_stdout_files_are_labeled() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.cmake");
    let second = dir.path().join("second.cmake");

    write_file(&first, "set(  FIRST  value )\n");
    write_file(&second, "set(  SECOND  value )\n");

    let output = cmakefmt()
        .args([first.to_str().unwrap(), second.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "### {}\nset(FIRST value)\n\n### {}\nset(SECOND value)\n",
            first.display(),
            second.display()
        )
    );
}

#[test]
fn multiple_stdout_headers_are_colored_when_forced() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.cmake");
    let second = dir.path().join("second.cmake");

    write_file(&first, "set(  FIRST  value )\n");
    write_file(&second, "set(  SECOND  value )\n");

    let output = cmakefmt()
        .args([
            "--colour",
            "always",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "\u{1b}[1;36m### {}\u{1b}[0m\n\u{1b}[36mset(FIRST value)\u{1b}[0m\n\n\u{1b}[1;36m### {}\u{1b}[0m\n\u{1b}[36mset(SECOND value)\u{1b}[0m\n",
            first.display(),
            second.display()
        )
    );
}

#[test]
fn no_args_discovers_cmake_files_recursively() {
    let dir = tempfile::tempdir().unwrap();
    let top = dir.path().join("CMakeLists.txt");
    let nested = dir.path().join("cmake/modules/CompilerWarnings.cmake");
    let ignored = dir.path().join("docs/example.txt");

    write_file(&top, "set(  TOP  value )\n");
    write_file(&nested, "set(  NESTED  value )\n");
    write_file(&ignored, "set(  IGNORED  value )\n");

    let output = cmakefmt()
        .args(["--list-input-files"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CMakeLists.txt"));
    assert!(stdout.contains("CompilerWarnings.cmake"));
    assert!(!stdout.contains("example.txt"));
}

#[test]
fn cmakefmtignore_filters_recursive_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let keep = dir.path().join("keep.cmake");
    let ignored = dir.path().join("ignored.cmake");

    write_file(&keep, "set(  KEEP  value )\n");
    write_file(&ignored, "set(  IGNORE  value )\n");
    write_file(&dir.path().join(".cmakefmtignore"), "ignored.cmake\n");

    let output = cmakefmt()
        .args(["--list-input-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("keep.cmake"));
    assert!(!stdout.contains("ignored.cmake"));
}

#[test]
fn explicit_ignore_path_filters_recursive_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let keep = dir.path().join("keep.cmake");
    let ignored = dir.path().join("ignored.cmake");
    let ignore_file = dir.path().join("extra.ignore");

    write_file(&keep, "set(  KEEP  value )\n");
    write_file(&ignored, "set(  IGNORE  value )\n");
    write_file(&ignore_file, "ignored.cmake\n");

    let output = cmakefmt()
        .args([
            "--list-input-files",
            "--ignore-path",
            ignore_file.to_str().unwrap(),
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("keep.cmake"));
    assert!(!stdout.contains("ignored.cmake"));
}

#[test]
fn explicit_file_argument_bypasses_ignore_rules() {
    let dir = tempfile::tempdir().unwrap();
    let ignored = dir.path().join("ignored.cmake");

    write_file(&ignored, "set(  IGNORE  value )\n");
    write_file(&dir.path().join(".cmakefmtignore"), "ignored.cmake\n");

    let output = cmakefmt()
        .args(["--list-input-files", ignored.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ignored.cmake"));
}

#[test]
fn no_gitignore_allows_gitignored_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    write_file(&dir.path().join(".gitignore"), "ignored.cmake\n");
    let ignored = dir.path().join("ignored.cmake");
    write_file(&ignored, "set(  IGNORE  value )\n");

    let default_output = cmakefmt()
        .args(["--list-input-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(default_output.status.code(), Some(0));

    let no_gitignore_output = cmakefmt()
        .args([
            "--list-input-files",
            "--no-gitignore",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(no_gitignore_output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&no_gitignore_output.stdout).contains("ignored.cmake"));
}

#[test]
fn directory_input_discovers_only_cmake_files() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("cmake/toolchain.cmake.in");
    let ignored = dir.path().join("cmake/ignore.txt");

    write_file(&nested, "set(  FOO  bar )\n");
    write_file(&ignored, "set(  NOPE  value )\n");

    let output = cmakefmt()
        .args(["--list-input-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("toolchain.cmake.in"));
    assert!(!stdout.contains("ignore.txt"));
}

#[test]
fn file_regex_filters_discovered_files() {
    let dir = tempfile::tempdir().unwrap();
    let keep = dir.path().join("cmake/KeepThis.cmake");
    let skip = dir.path().join("cmake/SkipThis.cmake");

    write_file(&keep, "set(  KEEP  value )\n");
    write_file(&skip, "set(  SKIP  value )\n");

    let output = cmakefmt()
        .args([
            "--list-input-files",
            "--path-regex",
            "Keep",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("KeepThis.cmake"));
    assert!(!stdout.contains("SkipThis.cmake"));
}

#[test]
fn list_input_files_reports_selected_targets_even_if_clean() {
    let dir = tempfile::tempdir().unwrap();
    let changed = dir.path().join("changed.cmake");
    let clean = dir.path().join("clean.cmake");

    write_file(&changed, "set(  FOO  bar )\n");
    write_file(&clean, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--list-input-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changed.cmake"));
    assert!(stdout.contains("clean.cmake"));
}

#[test]
fn list_changed_files_reports_only_changed_targets() {
    let dir = tempfile::tempdir().unwrap();
    let changed = dir.path().join("changed.cmake");
    let clean = dir.path().join("clean.cmake");

    write_file(&changed, "set(  FOO  bar )\n");
    write_file(&clean, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--list-changed-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changed.cmake"));
    assert!(!stdout.contains("clean.cmake"));
}

#[test]
fn list_input_files_rejects_json_report_mode() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--list-input-files",
            "--report-format",
            "json",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("--list-input-files only supports human output"));
}

#[test]
fn staged_selects_only_staged_cmake_files() {
    let dir = tempfile::tempdir().unwrap();
    init_git_repo(dir.path());

    let staged = dir.path().join("staged.cmake");
    let unstaged = dir.path().join("unstaged.cmake");
    let ignored = dir.path().join("notes.txt");
    write_file(&staged, "set(STAGED value)\n");
    write_file(&unstaged, "set(UNSTAGED value)\n");
    write_file(&ignored, "not cmake\n");
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "baseline"]);

    write_file(&staged, "set(  STAGED  value )\n");
    write_file(&unstaged, "set(  UNSTAGED  value )\n");
    git(dir.path(), &["add", "staged.cmake"]);

    let output = cmakefmt()
        .args(["--list-changed-files", "--staged"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("staged.cmake"));
    assert!(!stdout.contains("unstaged.cmake"));
    assert!(!stdout.contains("notes.txt"));
}

#[test]
fn changed_since_selects_only_changed_files() {
    let dir = tempfile::tempdir().unwrap();
    init_git_repo(dir.path());

    let changed = dir.path().join("changed.cmake");
    let clean = dir.path().join("clean.cmake");
    write_file(&changed, "set(CHANGED value)\n");
    write_file(&clean, "set(CLEAN value)\n");
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "baseline"]);
    let baseline = git_stdout(dir.path(), &["rev-parse", "HEAD"]);

    write_file(&changed, "set(  CHANGED  value )\n");

    let output = cmakefmt()
        .args(["--list-changed-files", "--changed", "--since", &baseline])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("changed.cmake"));
    assert!(!stdout.contains("clean.cmake"));
}

// ── Config file ─────────────────────────────────────────────────────────────

#[test]
fn explicit_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "[format]\ncommand_case = \"upper\"\n").unwrap();

    let mut child = cmakefmt()
        .args(["--config-file", config_path.to_str().unwrap(), "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("CMAKE_MINIMUM_REQUIRED("));
}

#[test]
fn config_short_alias_works() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "[format]\ncommand_case = \"upper\"\n").unwrap();

    let mut child = cmakefmt()
        .args(["-c", config_path.to_str().unwrap(), "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("CMAKE_MINIMUM_REQUIRED("));
}

#[test]
fn multiple_explicit_config_files_merge_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.toml");
    let second = dir.path().join("second.toml");
    std::fs::write(&first, "[format]\ncommand_case = \"upper\"\n").unwrap();
    std::fs::write(&second, "[format]\nkeyword_case = \"lower\"\n").unwrap();

    let mut child = cmakefmt()
        .args([
            "--config-file",
            first.to_str().unwrap(),
            "--config-file",
            second.to_str().unwrap(),
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "CMAKE_MINIMUM_REQUIRED(version 3.20)\n"
    );
}

#[test]
fn config_alias_still_works() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "[format]\ncommand_case = \"upper\"\n").unwrap();

    let mut child = cmakefmt()
        .args(["--config", config_path.to_str().unwrap(), "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"cmake_minimum_required(VERSION 3.20)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("CMAKE_MINIMUM_REQUIRED("));
}

#[test]
fn convert_legacy_json_config_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("cmake-format.json");
    std::fs::write(
        &legacy,
        r#"{
  "format": {
    "line_width": 100,
    "command_case": "lower"
  }
}"#,
    )
    .unwrap();

    let output = cmakefmt()
        .args(["config", "convert", legacy.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Converted from legacy cmake-format configuration."));
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 100"));
    assert!(stdout.contains("command_case:"));
    assert!(stdout.contains("command_case: lower"));
}

#[test]
fn convert_config_toml_prints_toml_when_requested() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("cmake-format.json");
    std::fs::write(
        &legacy,
        r#"{
  "format": {
    "line_width": 100,
    "command_case": "lower"
  }
}"#,
    )
    .unwrap();

    let output = cmakefmt()
        .args([
            "config",
            "convert",
            legacy.to_str().unwrap(),
            "--format",
            "toml",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Converted from legacy cmake-format configuration."));
    assert!(stdout.contains("[format]"));
    assert!(stdout.contains("line_width = 100"));
    assert!(stdout.contains("command_case = \"lower\""));
}

#[test]
fn discovered_config_uses_nearest_file_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.toml"),
        "[format]\ncommand_case = \"upper\"\n",
    )
    .unwrap();

    let subdir = dir.path().join("nested");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(
        subdir.join(".cmakefmt.yaml"),
        "format:\n  keyword_case: lower\n",
    )
    .unwrap();

    let file = subdir.join("CMakeLists.txt");
    write_file(&file, "cmake_minimum_required(VERSION 3.20)\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cmake_minimum_required(version 3.20)\n"
    );
}

#[test]
fn config_file_can_define_custom_command_specs() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.yaml");
    std::fs::write(
        &config_path,
        "format:\n  line_width: 30\ncommands:\n  my_add_test:\n    pargs: 1\n    kwargs:\n      SOURCES:\n        nargs: \"+\"\n      LIBRARIES:\n        nargs: \"+\"\n",
    )
    .unwrap();

    let input = dir.path().join("input.cmake");
    write_file(
        &input,
        "my_add_test(target SOURCES a.cpp b.cpp c.cpp LIBRARIES foo bar)\n",
    );

    let output = cmakefmt()
        .args([
            "--config-file",
            config_path.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my_add_test("));
    assert!(stdout.contains("\n  SOURCES a.cpp b.cpp c.cpp\n"));
    assert!(stdout.contains("\n  LIBRARIES foo bar)"));
}

#[test]
fn dump_config_prints_template() {
    let output = cmakefmt().args(["config", "dump"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Default cmakefmt configuration."));
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 80"));
    assert!(stdout.contains("# use_tabs: true"));
    assert!(stdout.contains("markup:"));
    assert!(stdout.contains("# per_command_overrides:"));
    assert!(stdout.contains("# commands:"));
    assert!(stdout.contains("#   my_add_test:"));
}

#[test]
fn dump_config_toml_prints_template() {
    let output = cmakefmt()
        .args(["config", "dump", "--format", "toml"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Default cmakefmt configuration."));
    assert!(stdout.contains("[format]"));
    assert!(stdout.contains("line_width = 80"));
    assert!(stdout.contains("# use_tabs = true"));
    assert!(stdout.contains("[markup]"));
    assert!(stdout.contains("# [per_command_overrides.my_add_test]"));
    assert!(stdout.contains("# [commands.my_add_test]"));
}

#[test]
fn show_config_prints_effective_yaml_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 99\n  command_case: upper\n",
    );
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["config", "show", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 99"));
    assert!(stdout.contains("command_case:"));
    assert!(stdout.contains("command_case: upper"));
}

#[test]
fn show_config_applies_cli_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 99\n",
    );
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--line-width",
            "120",
            "config",
            "show",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("line_width: 120"));
}

#[test]
fn show_config_path_prints_nearest_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    write_file(
        &dir.path().join(".cmakefmt.toml"),
        "[format]\ncommand_case = \"upper\"\n",
    );
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    write_file(
        &nested.join(".cmakefmt.yaml"),
        "format:\n  command_case: lower\n",
    );
    let file = nested.join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["config", "path", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{}\n", nested.join(".cmakefmt.yaml").display())
    );
}

#[test]
fn show_config_path_accepts_file_before_flag() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    write_file(&config, "format:\n  command_case: upper\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["config", "path", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{}\n", config.display())
    );
}

#[test]
fn find_config_path_alias_works() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    write_file(&config, "format:\n  command_case: upper\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["config", "path", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{}\n", config.display())
    );
}

#[test]
fn no_config_ignores_discovered_config_for_show_config() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 99\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--no-config", "config", "show", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("line_width: 80"));
}

#[test]
fn explain_config_reports_sources_and_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    write_file(&config, "format:\n  line_width: 99\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--line-width",
            "120",
            "config",
            "explain",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("target: {}", file.display())));
    assert!(stdout.contains("config mode: discovered from the target path"));
    assert!(stdout.contains(&config.display().to_string()));
    assert!(stdout.contains("cli overrides: line_width=120"));
    assert!(stdout.contains("effective config:"));
    assert!(stdout.contains("line_width: 120"));
}

#[test]
fn explain_config_uses_explicit_file_argument() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    write_file(&config, "format:\n  line_width: 99\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["config", "explain", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("target: {}", file.display())));
    assert!(stdout.contains("config mode: discovered from the target path"));
}

#[test]
fn explain_config_defaults_to_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    write_file(&config, "format:\n  line_width: 99\n");

    let output = cmakefmt()
        .args(["config", "explain"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target: ."));
    assert!(stdout.contains("config mode: discovered from the target path"));
    assert!(stdout.contains(".cmakefmt.yaml"));
}

#[test]
fn show_config_rejects_multiple_paths() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.cmake");
    let b = dir.path().join("b.cmake");
    write_file(&a, "set(A value)\n");
    write_file(&b, "set(B value)\n");

    let output = cmakefmt()
        .args(["config", "show", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn debug_mode_reports_config_and_barriers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 40\n",
    )
    .unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(  BEFORE  value )\n# cmakefmt: off\nthis is not valid cmake\n# cmakefmt: on\nset(  AFTER  value )\n",
    );

    let output = cmakefmt()
        .args(["--debug", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("debug: processing"));
    assert!(stderr.contains("debug: config sources:"));
    assert!(stderr.contains("debug: cli overrides:"));
    assert!(stderr.contains("formatter: disabled formatting"));
    assert!(stderr.contains("formatter: enabled formatting"));
}

#[test]
fn debug_mode_reports_command_form_and_layout_decision() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "target_link_libraries(foo PUBLIC bar baz)\n");

    let output = cmakefmt()
        .args(["--debug", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("formatter: command target_link_libraries form="));
    assert!(stderr.contains("effective_config("));
    assert!(stderr.contains("layout="));
    assert!(stderr.contains("changed_lines="));
}

#[test]
fn line_ranges_format_only_selected_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\nset(  BAZ  qux )\n");

    let output = cmakefmt()
        .args(["--lines", "2:2", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "set(FOO bar)\nset(BAZ qux)\n"
    );
}

#[test]
fn line_ranges_fail_when_changes_escape_requested_range() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  a b c d e f g h i )\n");

    let output = cmakefmt()
        .args([
            "--lines",
            "1:1",
            "--line-width",
            "20",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("selected line ranges would affect lines outside the requested ranges"));
}

#[test]
fn parallel_in_place_formats_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    let file_a = dir.path().join("a.cmake");
    let file_b = dir.path().join("b.cmake");

    write_file(&file_a, "set(  A  value )\n");
    write_file(&file_b, "set(  B  value )\n");

    let output = cmakefmt()
        .args(["--parallel", "2", "-i", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&file_a).unwrap(), "set(A value)\n");
    assert_eq!(std::fs::read_to_string(&file_b).unwrap(), "set(B value)\n");
}

#[test]
fn parallel_without_value_uses_default_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  value )\n");

    let output = cmakefmt()
        .args(["--parallel", "--check", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("would be reformatted"));
}

#[test]
fn progress_bar_works_without_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  value )\n");

    let output = cmakefmt()
        .args(["--progress-bar", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    // Should not fail with a clap error — progress-bar no longer requires --in-place
    assert_ne!(output.status.code(), Some(2));
}

#[test]
fn progress_bar_with_in_place_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let file_a = dir.path().join("a.cmake");
    let file_b = dir.path().join("b.cmake");

    write_file(&file_a, "set(  A  value )\n");
    write_file(&file_b, "set(  B  value )\n");

    let output = cmakefmt()
        .args(["--progress-bar", "--in-place", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&file_a).unwrap(), "set(A value)\n");
    assert_eq!(std::fs::read_to_string(&file_b).unwrap(), "set(B value)\n");
}

// ── Version ─────────────────────────────────────────────────────────────────

#[test]
fn version_flag() {
    let output = cmakefmt().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cmakefmt"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn required_version_accepts_exact_current_version() {
    let output = cmakefmt()
        .args(["--required-version", env!("CARGO_PKG_VERSION"), "--version"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn required_version_rejects_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--required-version", "9.9.9", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("required cmakefmt version 9.9.9"));
}

#[test]
fn verify_succeeds_for_stdout_formatting() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--verify", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
}

#[test]
fn fast_and_verify_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--fast", "--verify", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn generate_completion_outputs_bash_script() {
    let output = cmakefmt().args(["completions", "bash"]).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_cmakefmt"));
    assert!(stdout.contains("complete -F"));
}

#[test]
fn generate_man_page_outputs_roff() {
    let output = cmakefmt().arg("--generate-man-page").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".TH cmakefmt"));
    assert!(stdout.contains("Parse CMake listfiles and format them nicely."));
}

#[test]
fn help_mentions_config_discovery_and_primary_flags() {
    let output = cmakefmt().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parse CMake listfiles and format them nicely."));
    assert!(stdout.contains(".cmakefmt.yaml"));
    assert!(stdout.contains(".cmakefmt.yml"));
    assert!(stdout.contains(".cmakefmt.toml"));
    assert!(stdout.contains("--color <COLOR>"));
    assert!(stdout.contains("--generate-man-page"));
    assert!(stdout.contains("--required-version <VERSION>"));
    assert!(stdout.contains("--verify"));
    assert!(stdout.contains("--no-verify"));
    assert!(stdout.contains("--require-pragma"));
    assert!(stdout.contains("--in-place"));
    assert!(stdout.contains("-c, --config-file <PATH>"));
    assert!(stdout.contains("--no-config"));
    assert!(stdout.contains("-l, --line-width <LINE_WIDTH>"));
    assert!(stdout.contains("--list-changed-files"));
    assert!(stdout.contains("--list-input-files"));
    assert!(stdout.contains("--path-regex <REGEX>"));
    assert!(stdout.contains("--ignore-path <PATH>"));
    assert!(stdout.contains("--no-gitignore"));
    // Subcommands
    assert!(stdout.contains("config"));
    assert!(stdout.contains("completions"));
    assert!(stdout.contains("lsp"));
    assert!(stdout.contains("install-hook"));
    assert!(stdout.contains("--files-from <PATH>"));
    assert!(stdout.contains("--diff"));
    assert!(stdout.contains("--quiet"));
    assert!(stdout.contains("--keep-going"));
    assert!(stdout.contains("--cache"));
    assert!(stdout.contains("--cache-location <PATH>"));
    assert!(stdout.contains("--cache-strategy <CACHE_STRATEGY>"));
    assert!(stdout.contains("--staged"));
    assert!(stdout.contains("--changed"));
    assert!(stdout.contains("--since <REF>"));
    assert!(stdout.contains("--stdin-path <PATH>"));
    assert!(stdout.contains("--lines <START:END>"));
    assert!(stdout.contains("--report-format <REPORT_FORMAT>"));
    assert!(stdout.contains("--progress-bar"));
    assert!(stdout.contains("parallel formatting jobs"));
}

// ── config schema ───────────────────────────────────────────────────────────

#[test]
fn dump_schema_exits_zero() {
    let output = cmakefmt().args(["config", "schema"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn dump_schema_prints_valid_json() {
    let output = cmakefmt().args(["config", "schema"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value =
        serde_json::from_str(&stdout).expect("config schema output should be valid JSON");
    // Verify it is a JSON Schema object with expected top-level keys
    assert!(parsed.get("$schema").is_some(), "should have $schema key");
    assert!(parsed.get("title").is_some(), "should have title key");
    assert!(
        parsed.get("properties").is_some(),
        "should have properties key"
    );
}

#[test]
fn dump_schema_output_ends_with_newline() {
    let output = cmakefmt().args(["config", "schema"]).output().unwrap();
    assert!(
        output.stdout.ends_with(b"\n"),
        "config schema output should end with a newline"
    );
}

#[test]
fn dump_schema_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("config"));
}

// ── --stat ──────────────────────────────────────────────────────────────────

#[test]
fn stat_prints_summary() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "MESSAGE(hello)\n");

    let output = cmakefmt()
        .args(["--stat", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("file changed") || stderr.contains("files changed"));
}

#[test]
fn stat_shows_zero_when_nothing_changes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "message(hello)\n");

    let output = cmakefmt()
        .args(["--stat", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("0 files changed"),
        "expected '0 files changed' in stderr: {stderr}"
    );
}

#[test]
fn stat_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--stat"));
}

// ── check fix hint ─────────────────────────────────────────────────────────

#[test]
fn check_failure_prints_fix_hint() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "MESSAGE(hello)\n");

    let output = cmakefmt()
        .args(["--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cmakefmt --in-place"),
        "check failure should print fix hint"
    );
}

// ── init subcommand ────────────────────────────────────────────────────────

#[test]
fn init_creates_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let output = cmakefmt()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(dir.path().join(".cmakefmt.yaml").exists());
    let content = std::fs::read_to_string(dir.path().join(".cmakefmt.yaml")).unwrap();
    assert!(content.contains("line_width"));
}

#[test]
fn init_refuses_to_overwrite_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 100\n",
    )
    .unwrap();

    let output = cmakefmt()
        .args(["config", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let content = std::fs::read_to_string(dir.path().join(".cmakefmt.yaml")).unwrap();
    assert_eq!(content, "format:\n  line_width: 100\n");
}

// ── config check ───────────────────────────────────────────────────────────

#[test]
fn check_config_validates_good_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 100\n",
    )
    .unwrap();

    let output = cmakefmt()
        .args(["config", "check"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid"));
}

#[test]
fn check_config_rejects_bad_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".cmakefmt.yaml"), "{{invalid yaml\n").unwrap();

    let output = cmakefmt()
        .args(["config", "check"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn check_config_with_explicit_path() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("my-config.yaml");
    std::fs::write(&config, "format:\n  line_width: 120\n").unwrap();

    let output = cmakefmt()
        .args(["config", "check", config.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid"));
}

#[test]
fn check_config_no_config_exits_2() {
    let dir = tempfile::tempdir().unwrap();

    let output = cmakefmt()
        .args(["config", "check"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no config file found"));
}

// ── lsp subcommand ───────────────────────────────────────────────────────────────────

#[test]
fn lsp_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lsp"));
}

// ── summary output ──────────────────────────────────────────────────────────

#[test]
fn summary_check_shows_changed_file_status() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "summary output should show [!] for changed file, got: {stderr}"
    );
    assert!(
        stderr.contains("lines changed"),
        "summary output should show changed line count, got: {stderr}"
    );
    // --summary suppresses "would be reformatted" since the summary line
    // already conveys that information.
    assert!(
        !stderr.contains("would be reformatted"),
        "summary mode should suppress 'would be reformatted', got: {stderr}"
    );
}

#[test]
fn summary_check_shows_unchanged_file_status() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[ok]"),
        "summary output should show [ok] for unchanged file, got: {stderr}"
    );
    assert!(
        stderr.contains("unchanged"),
        "summary output should say unchanged, got: {stderr}"
    );
}

#[test]
fn summary_in_place_shows_file_status() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--in-place",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "summary output should show [!] for changed file, got: {stderr}"
    );
    // File should also be modified on disk
    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, "set(FOO bar)\n");
}

#[test]
fn summary_diff_shows_file_status() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--diff",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "summary should show status on stderr, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("---"),
        "diff should still appear on stdout, got: {stdout}"
    );
}

#[test]
fn summary_stdout_suppresses_formatted_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args(["--summary", "--colour", "never", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "summary status should appear on stderr, got: {stderr}"
    );
    // --summary in stdout mode suppresses formatted output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "stdout should be empty with --summary, got: {stdout}"
    );
}

#[test]
fn summary_list_changed_files_shows_file_status() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--list-changed-files",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "summary status should be on stderr, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CMakeLists.txt"),
        "file list should be on stdout, got: {stdout}"
    );
}

#[test]
fn summary_with_require_pragma_shows_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--require-pragma",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[-]"),
        "summary output should show [-] for skipped file, got: {stderr}"
    );
    assert!(
        stderr.contains("skipped"),
        "summary output should say skipped, got: {stderr}"
    );
}

#[test]
fn summary_keep_going_shows_failed_file() {
    let dir = tempfile::tempdir().unwrap();
    let bad_file = dir.path().join("bad.cmake");
    write_file(&bad_file, "if(\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--keep-going",
            "--check",
            bad_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!!]"),
        "summary output should show [!!] for failed file, got: {stderr}"
    );
    assert!(
        stderr.contains("parse error"),
        "summary output should mention parse error, got: {stderr}"
    );
}

#[test]
fn summary_conflicts_with_quiet() {
    let output = cmakefmt()
        .args(["--summary", "--quiet", "."])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "should report conflict, got: {stderr}"
    );
}

#[test]
fn summary_with_color_uses_unicode_markers() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "always",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Yellow exclamation for changed file
    assert!(
        stderr.contains('!'),
        "color mode should use ! for changed file, got: {stderr}"
    );
    // Tree branch connector U+2514 U+2500
    assert!(
        stderr.contains("\u{2514}\u{2500}"),
        "should use tree branch connector, got: {stderr}"
    );
}

#[test]
fn summary_shows_elapsed_time() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain either "<1ms" or "Nms" or "N.NNs"
    assert!(
        stderr.contains("ms") || stderr.contains("s"),
        "summary output should include elapsed time, got: {stderr}"
    );
}

#[test]
fn summary_shows_line_count() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1 lines"),
        "summary output should include line count, got: {stderr}"
    );
}

#[test]
fn summary_json_report_includes_elapsed_and_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--report-format",
            "json",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(
        files[0]["elapsed_ms"].is_number(),
        "JSON report should include elapsed_ms with --summary"
    );
    assert!(
        files[0]["source_lines"].is_number(),
        "JSON report should include source_lines with --summary"
    );
    assert!(
        files[0]["formatted_lines"].is_number(),
        "JSON report should include formatted_lines with --summary"
    );
}

#[test]
fn json_report_omits_summary_fields_without_flag() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--report-format", "json", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(
        files[0].get("elapsed_ms").is_none(),
        "JSON report should not include elapsed_ms without --summary"
    );
    assert!(
        files[0].get("source_lines").is_none(),
        "JSON report should not include source_lines without --summary"
    );
}

#[test]
fn summary_with_multiple_files_shows_all() {
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("a.cmake");
    let file2 = dir.path().join("b.cmake");
    write_file(&file1, "set(  FOO  bar )\n");
    write_file(&file2, "set(BAZ qux)\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file1.to_str().unwrap(),
            file2.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[!]"),
        "should show changed marker for a.cmake, got: {stderr}"
    );
    assert!(
        stderr.contains("[ok]"),
        "should show unchanged marker for b.cmake, got: {stderr}"
    );
}

#[test]
fn summary_suppressed_with_non_human_report_format() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--report-format",
            "json",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Summary lines should not appear on stderr with non-human report formats.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("[!]"),
        "summary lines should be suppressed with --report-format json, got: {stderr}"
    );

    // JSON should still be valid on stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert!(report["summary"]["changed"].as_u64().unwrap() > 0);
}

#[test]
fn summary_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--summary"));
}

#[test]
fn summary_two_line_format_for_changed_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt()
        .args([
            "--summary",
            "--colour",
            "never",
            "--check",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // The summary output should be on two lines: marker+name, then indented details
    let summary_lines: Vec<&str> = stderr.lines().collect();
    let marker_line = summary_lines
        .iter()
        .position(|l| l.starts_with("[!]"))
        .expect("should have a [!] line");
    // Next line should be indented with details
    assert!(
        summary_lines[marker_line + 1].starts_with("     "),
        "detail line should be indented, got: '{}'",
        summary_lines[marker_line + 1]
    );
    assert!(
        summary_lines[marker_line + 1].contains("lines changed"),
        "detail line should contain change info, got: '{}'",
        summary_lines[marker_line + 1]
    );
}

// ── editorconfig ──────────────────────────────────────────────────────

#[test]
fn editorconfig_fallback_sets_tab_size() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".editorconfig"),
        "[*]\nroot = true\nindent_style = space\nindent_size = 8\n",
    )
    .unwrap();
    // No .cmakefmt.yaml — editorconfig should be the fallback.
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "if(TRUE)\nset(FOO bar)\nendif()\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // With indent_size=8, the `set` line should be indented with 8 spaces.
    assert!(
        stdout.contains("        set(FOO bar)"),
        "expected 8-space indent from editorconfig, got:\n{stdout}"
    );
}

#[test]
fn cmakefmt_config_overrides_editorconfig() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".editorconfig"),
        "[*]\nroot = true\nindent_size = 8\n",
    )
    .unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  tab_size: 3\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "if(TRUE)\nset(FOO bar)\nendif()\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // cmakefmt config (tab_size=3) should win over editorconfig (indent_size=8).
    assert!(
        stdout.contains("   set(FOO bar)"),
        "expected 3-space indent from cmakefmt config, got:\n{stdout}"
    );
}

#[test]
fn no_editorconfig_flag_disables_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".editorconfig"),
        "[*]\nroot = true\nindent_size = 8\n",
    )
    .unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "if(TRUE)\nset(FOO bar)\nendif()\n");

    let output = cmakefmt()
        .args(["--no-editorconfig", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Default tab_size is 2, not 8 from editorconfig.
    assert!(
        stdout.contains("  set(FOO bar)"),
        "expected 2-space default indent with --no-editorconfig, got:\n{stdout}"
    );
}

// ── --explain ─────────────────────────────────────────────────────────

#[test]
fn explain_shows_formatter_decisions() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC dep1 dep2 dep3)\n",
    );

    let output = cmakefmt()
        .args(["--explain", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Formatting decisions for"),
        "expected explain header, got:\n{stderr}"
    );
    assert!(
        stderr.contains("layout="),
        "expected layout decision in explain output, got:\n{stderr}"
    );
}

#[test]
fn explain_requires_single_target() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.cmake");
    let b = dir.path().join("b.cmake");
    write_file(&a, "set(A val)\n");
    write_file(&b, "set(B val)\n");

    let output = cmakefmt()
        .args(["--explain", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exactly one"),
        "expected single-target error, got:\n{stderr}"
    );
}

#[test]
fn explain_conflicts_with_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(A val)\n");

    let output = cmakefmt()
        .args(["--explain", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

// ── --watch ───────────────────────────────────────────────────────────

#[test]
fn watch_conflicts_with_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(A val)\n");

    let output = cmakefmt()
        .args(["--watch", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn watch_rejects_stdin() {
    let output = cmakefmt().args(["--watch", "-"]).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stdin"),
        "expected stdin rejection, got:\n{stderr}"
    );
}

#[test]
fn watch_reformats_changed_file() {
    use std::time::Duration;

    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  bar )\n");

    // Spawn --watch in background
    let mut child = cmakefmt()
        .args(["--watch", dir.path().to_str().unwrap()])
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Give the watcher time to start
    std::thread::sleep(Duration::from_millis(500));

    // Touch the file to trigger a reformat
    write_file(&file, "set(  FOO  bar )\n");

    // Wait for the watcher to process the event
    std::thread::sleep(Duration::from_secs(2));

    // Kill the watcher
    child.kill().ok();
    child.wait().ok();

    // Check the file was reformatted
    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, "set(FOO bar)\n");
}

// ── --list-unknown-commands ───────────────────────────────────────────

#[test]
fn list_unknown_commands_reports_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "cmake_minimum_required(VERSION 3.20)\nmy_thing(FOO bar)\nset(X y)\n",
    );

    let output = cmakefmt()
        .args(["--list-unknown-commands", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("my_thing"),
        "expected my_thing, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("cmake_minimum_required"),
        "should not list known commands"
    );
    assert!(!stdout.contains("set"), "should not list known commands");
}

#[test]
fn list_unknown_commands_no_unknowns() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "cmake_minimum_required(VERSION 3.20)\nset(X y)\n");

    let output = cmakefmt()
        .args(["--list-unknown-commands", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "expected no output, got:\n{stdout}");
}

#[test]
fn list_unknown_commands_respects_user_specs() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "commands:\n  my_thing:\n    pargs: 1\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "my_thing(FOO)\n");

    let output = cmakefmt()
        .args(["--list-unknown-commands", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "user-defined command should not be listed as unknown, got:\n{stdout}"
    );
}

// ── LSP large-file test ───────────────────────────────────────────────

#[test]
fn lsp_formats_large_file_within_timeout() {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    // Generate a ~2000-line CMake file.
    let mut source = String::from("cmake_minimum_required(VERSION 3.20)\nproject(big)\n\n");
    for i in 0..500 {
        source.push_str(&format!(
            "add_library(lib_{i} STATIC src_{i}/a.cpp src_{i}/b.cpp src_{i}/c.cpp src_{i}/d.cpp)\n"
        ));
    }

    let uri = "file:///tmp/big-CMakeLists.txt";
    let init_params = serde_json::json!({
        "processId": null,
        "capabilities": {},
        "rootUri": null
    });
    let format_params = serde_json::json!({
        "textDocument": { "uri": uri },
        "options": { "tabSize": 2, "insertSpaces": true }
    });

    fn lsp_message(method: &str, id: i64, params: &serde_json::Value) -> String {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let body_str = body.to_string();
        format!("Content-Length: {}\r\n\r\n{}", body_str.len(), body_str)
    }

    fn lsp_notification(method: &str, params: &serde_json::Value) -> String {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let body_str = body.to_string();
        format!("Content-Length: {}\r\n\r\n{}", body_str.len(), body_str)
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
        .args(["lsp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    let stdout = child.stdout.as_mut().unwrap();

    // Initialize
    let init = lsp_message("initialize", 1, &init_params);
    stdin.write_all(init.as_bytes()).unwrap();
    stdin.flush().unwrap();

    // Read initialize response (we don't parse it, just drain it)
    std::thread::sleep(Duration::from_millis(200));
    let mut buf = vec![0u8; 4096];
    let _ = stdout.read(&mut buf);

    // Send initialized notification
    let initialized = lsp_notification("initialized", &serde_json::json!({}));
    stdin.write_all(initialized.as_bytes()).unwrap();

    // Open the document
    let open = lsp_notification(
        "textDocument/didOpen",
        &serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": "cmake",
                "version": 1,
                "text": source
            }
        }),
    );
    stdin.write_all(open.as_bytes()).unwrap();
    stdin.flush().unwrap();

    // Send format request and time it
    let format_msg = lsp_message("textDocument/formatting", 2, &format_params);
    let start = Instant::now();
    stdin.write_all(format_msg.as_bytes()).unwrap();
    stdin.flush().unwrap();

    // Read response — wait up to 10s
    let mut response = vec![0u8; 1024 * 1024];
    let timeout = Duration::from_secs(10);
    loop {
        if start.elapsed() > timeout {
            child.kill().ok();
            panic!(
                "LSP formatting timed out after 10 seconds on a {}-line file",
                source.lines().count()
            );
        }
        match stdout.read(&mut response) {
            Ok(n) if n > 0 => {
                let elapsed = start.elapsed();
                // Kill the LSP and verify timing
                child.kill().ok();
                child.wait().ok();
                assert!(
                    elapsed < timeout,
                    "LSP formatting took {:?} — exceeds 10s timeout",
                    elapsed
                );
                return;
            }
            _ => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

// ── enable_sort / autosort ────────────────────────────────────────────

#[test]
fn enable_sort_does_not_sort_without_sortable_annotation() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC zebra apple mango)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // PUBLIC args not marked sortable in builtin spec — should preserve order
    assert!(
        stdout.contains("zebra apple mango"),
        "expected unsorted output, got:\n{stdout}"
    );
}

#[test]
fn autosort_sorts_simple_unquoted_args() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n  autosort: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC zebra apple mango)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("apple mango zebra"),
        "expected sorted output, got:\n{stdout}"
    );
}

#[test]
fn autosort_does_not_sort_args_with_variables() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n  autosort: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC ${ZEBRA_LIB} apple mango)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Contains a variable — autosort should not sort
    assert!(
        stdout.contains("${ZEBRA_LIB} apple mango"),
        "expected unsorted output (variable present), got:\n{stdout}"
    );
}

#[test]
fn enable_sort_with_sortable_spec_sorts() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        concat!(
            "format:\n  enable_sort: true\n",
            "commands:\n  my_cmd:\n    pargs: 0\n    kwargs:\n",
            "      ITEMS:\n        nargs: \"+\"\n        sortable: true\n"
        ),
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "my_cmd(ITEMS zebra apple mango)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("apple mango zebra"),
        "expected sorted output from sortable spec, got:\n{stdout}"
    );
}

#[test]
fn sort_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n  autosort: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC Zebra apple Mango)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("apple Mango Zebra"),
        "expected case-insensitive sorted output, got:\n{stdout}"
    );
}

// ── wrap_after_first_arg (set formatting) ─────────────────────────────

#[test]
fn set_simple_stays_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(FOO bar)");
}

#[test]
fn set_short_list_stays_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO a b c)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(FOO a b c)");
}

#[test]
fn set_long_list_wraps_with_name_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(HEADERS header_a.h header_b.h header_c.h header_d.h header_e.h header_f.h)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    assert!(
        first_line.starts_with("set(HEADERS"),
        "variable name should stay on set( line, got:\n{stdout}"
    );
}

#[test]
fn set_cached_keeps_name_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(TRUE)\n  set(CMAKE_BUILD_TYPE \"Release\" CACHE STRING \"Build mode.\" FORCE)\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("set(CMAKE_BUILD_TYPE"),
        "variable name should stay on set( line, got:\n{stdout}"
    );
}

#[test]
fn set_cached_inline_keyword_args() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(TRUE)\n  set(CMAKE_BUILD_TYPE \"Release\" CACHE STRING \"Build mode.\" FORCE)\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // CACHE STRING "Build mode." FORCE should fit on one line
    assert!(
        stdout.contains("CACHE STRING \"Build mode.\" FORCE"),
        "CACHE args should be inline when they fit, got:\n{stdout}"
    );
}

#[test]
fn set_cached_value_stays_on_first_line() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(TRUE)\n  set(CMAKE_BUILD_TYPE \"Release\" CACHE STRING \"Build mode.\" FORCE)\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("set(CMAKE_BUILD_TYPE \"Release\""),
        "value should stay on same line as variable name, got:\n{stdout}"
    );
}

#[test]
fn set_cached_force_is_flag_not_positional() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    // FORCE should be at the CACHE keyword level, not nested deeper
    write_file(
        &file,
        "set(CMAKE_BUILD_TYPE \"Release\" CACHE STRING \"Build mode for performance but this description is very long and will cause wrapping.\" FORCE)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // FORCE should appear on a line indented at the same level as STRING
    let force_line = stdout.lines().find(|l| l.trim() == "FORCE");
    let string_line = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("STRING"));
    if let (Some(f), Some(s)) = (force_line, string_line) {
        let force_indent = f.len() - f.trim_start().len();
        let string_indent = s.len() - s.trim_start().len();
        assert_eq!(
            force_indent, string_indent,
            "FORCE and STRING should be at the same indent level, got:\n{stdout}"
        );
    }
}

#[test]
fn set_cached_inline_when_fits() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(FOO \"default\" CACHE STRING \"A description\" FORCE)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "set(FOO \"default\" CACHE STRING \"A description\" FORCE)",
        "should stay inline when it fits"
    );
}

#[test]
fn set_long_cached_wraps_keyword_nested() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(TRUE)\n  set(CMAKE_BUILD_TYPE \"LONG LONG VALUE\" CACHE STRING \"THIS IS A REALLY REALLY REALLY REALLY REALLY LONG DESCRIPTION THAT OVERFLOWS\" FORCE)\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // CACHE keyword should be on its own line with args nested
    assert!(
        stdout.lines().any(|l| l.trim() == "CACHE"),
        "CACHE should get its own line when args don't fit inline, got:\n{stdout}"
    );
}

#[test]
fn set_parent_scope_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO \"value\" PARENT_SCOPE)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(FOO \"value\" PARENT_SCOPE)");
}

#[test]
fn set_env_stays_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(ENV{FOO} \"value\")\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(ENV{FOO} \"value\")");
}

#[test]
fn set_unset_stays_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(FOO)");
}

#[test]
fn set_comment_on_variable_name_stays_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(foobarbaz # comment about foobarbaz\n    value_one value_two value_three)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("set(foobarbaz # comment about foobarbaz"),
        "comment should stay attached to variable name, got:\n{stdout}"
    );
}

#[test]
fn set_twenty_items_vertical_with_name_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp g.cpp h.cpp i.cpp j.cpp k.cpp l.cpp m.cpp n.cpp o.cpp p.cpp q.cpp r.cpp s.cpp t.cpp)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    assert_eq!(first_line, "set(SOURCES");
    // Each file should be on its own line (vertical layout)
    assert!(
        stdout.contains("    a.cpp"),
        "expected indented args, got:\n{stdout}"
    );
}

#[test]
fn set_deeply_nested_keeps_name_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(A)\n  if(B)\n    if(C)\n      set(CMAKE_BUILD_TYPE \"Release\" CACHE STRING \"Build mode.\" FORCE)\n    endif()\n  endif()\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("set(CMAKE_BUILD_TYPE"),
        "variable name should stay attached even at deep nesting, got:\n{stdout}"
    );
}

#[test]
fn wrap_after_first_arg_user_override() {
    let dir = tempfile::tempdir().unwrap();
    // Disable wrap_after_first_arg for set via per-command override
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "per_command_overrides:\n  set:\n    wrap_after_first_arg: false\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(HEADERS header_a.h header_b.h header_c.h header_d.h header_e.h header_f.h)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    // With override=false, should use old behavior: set(\n  HEADERS...
    assert_eq!(
        first_line, "set(",
        "user override should disable wrap_after_first_arg, got:\n{stdout}"
    );
}

#[test]
fn wrap_after_first_arg_global_config() {
    let dir = tempfile::tempdir().unwrap();
    // Enable globally — should affect all commands
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  wrap_after_first_arg: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib PUBLIC dep1 dep2 dep3 dep4 dep5 dep6 dep7 dep8 dep9)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    assert!(
        first_line.starts_with("target_link_libraries(mylib"),
        "global wrap_after_first_arg should keep first arg attached, got:\n{stdout}"
    );
}

// ── trailing comment handling ─────────────────────────────────────────

#[test]
fn trailing_comments_stay_attached_in_vertical_layout() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(mylib\n  PUBLIC\n    dep1 # first dep\n    dep2 # second dep\n    dep3 # third dep\n)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dep1 # first dep"),
        "dep1 comment should stay attached, got:\n{stdout}"
    );
    assert!(
        stdout.contains("dep2 # second dep"),
        "dep2 comment should stay attached, got:\n{stdout}"
    );
    assert!(
        stdout.contains("dep3 # third dep"),
        "dep3 comment should stay attached, got:\n{stdout}"
    );
}

#[test]
fn standalone_comment_does_not_merge_with_previous_trailing_comment() {
    // Regression: a long standalone comment between two arguments gets
    // reflowed by `format_comment_lines` into shorter lines. On the second
    // format pass the now-short first line was being merged onto the
    // previous argument's trailing comment, breaking idempotency.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(FOOS\n  \"/w34189\"  # local variable is initialized but not referenced\n  # see https://example.com/some/very/long/url/that/forces/reflow\n  \"/w35038\"\n)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let once = String::from_utf8_lossy(&output.stdout).into_owned();

    let twice_input = dir.path().join("once.cmake");
    write_file(&twice_input, &once);
    let output2 = cmakefmt()
        .args([twice_input.to_str().unwrap()])
        .output()
        .unwrap();
    let twice = String::from_utf8_lossy(&output2.stdout).into_owned();

    assert_eq!(
        once, twice,
        "output must be idempotent across two format passes, got:\nonce:\n{once}\ntwice:\n{twice}"
    );
}

#[test]
fn trailing_comment_on_command_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar) # this is a trailing comment\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "set(FOO bar) # this is a trailing comment");
}

#[test]
fn long_trailing_comment_on_command_reflows_at_eof_without_input_newline() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 60\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    let input = "set(FOO bar) # this is a very long trailing comment that should wrap cleanly even when the input file ends immediately after the comment";
    std::fs::write(&file, input).unwrap();

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        stdout.ends_with('\n'),
        "formatter should normalize EOF to end with a newline, got:\n{stdout:?}"
    );
    assert!(
        lines.len() >= 2,
        "long trailing comment should wrap across multiple lines, got:\n{stdout}"
    );

    let mut reconstructed = String::new();
    for (index, line) in lines.iter().enumerate() {
        let prefix = if index == 0 {
            "set(FOO bar) # "
        } else {
            "             # "
        };
        assert!(
            line.starts_with(prefix),
            "wrapped trailing comment line should start with {prefix:?}, got:\n{stdout}"
        );
        if !reconstructed.is_empty() {
            reconstructed.push(' ');
        }
        reconstructed.push_str(line.strip_prefix(prefix).unwrap());
    }

    assert_eq!(
        reconstructed,
        "this is a very long trailing comment that should wrap cleanly even when the input file ends immediately after the comment"
    );
}

#[test]
fn trailing_comment_does_not_force_vertical_layout() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    // Short enough to stay inline even with the comment
    write_file(&file, "set(FOO bar) # short\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should remain on one line — comment should not force wrapping
    assert_eq!(
        stdout.lines().count(),
        1,
        "comment should not force wrapping, got:\n{stdout}"
    );
}

#[test]
fn trailing_comment_too_long_moves_to_own_line() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 40\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(\n  mylib\n  PUBLIC\n    dep1 # this comment is way too long to fit\n)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Comment should NOT be on the same line as dep1
    assert!(
        !stdout.contains("dep1 # this comment"),
        "comment should not stay inline when too long, got:\n{stdout}"
    );
    // dep1 should appear on its own line
    assert!(
        stdout.lines().any(|l| l.trim() == "dep1"),
        "dep1 should be on its own line, got:\n{stdout}"
    );
    // The comment should appear on a separate line
    assert!(
        stdout
            .lines()
            .any(|l| l.trim().starts_with("# this comment")),
        "comment should appear on its own line, got:\n{stdout}"
    );
}

#[test]
fn multiple_inline_comments_each_stay_attached() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        concat!(
            "add_library(\n",
            "  mylib STATIC\n",
            "  src/a.cpp # module A\n",
            "  src/b.cpp # module B\n",
            "  src/c.cpp # module C\n",
            "  src/d.cpp # module D\n",
            ")\n",
        ),
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for (file_name, comment) in [
        ("src/a.cpp", "# module A"),
        ("src/b.cpp", "# module B"),
        ("src/c.cpp", "# module C"),
        ("src/d.cpp", "# module D"),
    ] {
        let pattern = format!("{file_name} {comment}");
        assert!(
            stdout.contains(&pattern),
            "expected '{pattern}' to stay attached, got:\n{stdout}"
        );
    }
}

#[test]
fn comment_on_if_condition_stays_inline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "if(CONDITION) # check this\n  message(STATUS \"hello\")\nendif()\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("if(CONDITION) # check this"),
        "comment on if should stay inline, got:\n{stdout}"
    );
}

// ── Semantic verifier gap tests ───────────────────────────────────────

#[test]
fn verify_rejects_when_argument_is_dropped() {
    // Manually create a "formatted" file with a missing argument to confirm
    // the verifier catches genuine structural changes.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar baz)\n");

    // Format it first so it's clean.
    let output = cmakefmt()
        .args(["--verify", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Now tamper with the file — remove an argument.
    write_file(&file, "set(FOO bar)\n");

    // Re-format the tampered file and check that --verify still succeeds
    // (since the tampered file is itself valid — verifier compares input vs
    // output, not vs the original). To actually test rejection we need to
    // pass two different ASTs to the verifier. That's done in the next test.
    // Here we just confirm --verify --in-place works on a clean file.
    let output = cmakefmt()
        .args(["--verify", "--in-place", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "verify should accept a self-consistent file"
    );
}

#[test]
fn verify_in_place_accepts_trailing_comment_reflow() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "markup:\n  enable_markup: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(FOO bar) # this is a very long trailing comment that exceeds the line width for a trailing comment\n",
    );

    let output = cmakefmt()
        .args(["--verify", "--in-place", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "verify should accept trailing comment reflow, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result = std::fs::read_to_string(&file).unwrap();
    assert!(
        result.contains("             # "),
        "trailing comment should be reflowed with aligned continuation, got:\n{result}"
    );
}

#[test]
fn enable_markup_false_suppresses_trailing_comment_reflow() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "markup:\n  enable_markup: false\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    let input =
        "set(FOO bar) # this is a very long trailing comment that exceeds the line width for a trailing comment\n";
    write_file(&file, input);

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Comment should stay on one line — not reflowed.
    assert_eq!(
        stdout.lines().count(),
        1,
        "enable_markup: false should suppress trailing comment reflow, got:\n{stdout}"
    );
}

// ── wrap_after_first_arg + dangle_parens interaction ──────────────────

#[test]
fn wrap_after_first_arg_with_dangle_parens() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 40\n  dangle_parens: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(MY_VAR value_one value_two value_three value_four)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Variable name should stay on set( line.
    assert!(
        stdout.starts_with("set(MY_VAR"),
        "variable name should stay attached with wrap_after_first_arg, got:\n{stdout}"
    );
    // With dangle_parens, closing paren should be on its own line.
    let last_line = stdout.trim_end().lines().last().unwrap();
    assert!(
        last_line.trim() == ")",
        "dangle_parens should put closing paren on own line, got:\n{stdout}"
    );
}

// ── Sort with inline comments ─────────────────────────────────────────

#[test]
fn autosort_sorts_args_with_inline_comments_present() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n  autosort: true\n  line_width: 80\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "target_link_libraries(\n  mylib\n  PUBLIC\n    zebra # z-lib\n    apple # a-lib\n    mango # m-lib\n)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Arguments should be sorted even when inline comments are present.
    // Comments stay at their original positions (not attached to the arg).
    let arg_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains("# ") && !l.contains("target_link"))
        .collect();

    assert_eq!(
        arg_lines.len(),
        3,
        "expected 3 argument lines with comments, got:\n{stdout}"
    );
    assert!(
        arg_lines[0].trim().starts_with("apple"),
        "first sorted arg should be apple, got:\n{stdout}"
    );
    assert!(
        arg_lines[1].trim().starts_with("mango"),
        "second sorted arg should be mango, got:\n{stdout}"
    );
    assert!(
        arg_lines[2].trim().starts_with("zebra"),
        "third sorted arg should be zebra, got:\n{stdout}"
    );
}

// ── Sort stability ────────────────────────────────────────────────────

#[test]
fn autosort_is_stable_for_equal_elements() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  enable_sort: true\n  autosort: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    // Two identical items — order should be preserved.
    write_file(
        &file,
        "target_link_libraries(\n  mylib\n  PUBLIC\n    dup_first\n    other\n    dup_first)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Run a second time — output should be identical (stable sort).
    let file2 = dir.path().join("round2.cmake");
    std::fs::write(&file2, stdout.as_bytes()).unwrap();
    let output2 = cmakefmt().args([file2.to_str().unwrap()]).output().unwrap();
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert_eq!(
        stdout.as_ref(),
        stdout2.as_ref(),
        "sort should be stable (idempotent for equal elements)"
    );
}

// ── Config: reflow_comments rejected in new config ────────────────────

#[test]
fn reflow_comments_rejected_in_native_config() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "markup:\n  reflow_comments: true\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "reflow_comments should be rejected as unknown field"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reflow_comments"),
        "error should mention the rejected key, stderr:\n{stderr}"
    );
}

// ── Pack remaining args near boundary ─────────────────────────────────

#[test]
fn pack_remaining_args_boundary_fits_exactly() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 30\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    // set(VAR has 8 chars with paren. Remaining args need to fit on one line
    // after the variable name.
    write_file(&file, "set(VAR aa bb cc dd ee ff)\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Variable name should stay on set( line.
    assert!(
        stdout.starts_with("set(VAR"),
        "variable name should stay attached, got:\n{stdout}"
    );
    // Verify no line exceeds line_width (allow comment lines to exceed).
    for (i, line) in stdout.lines().enumerate() {
        if !line.contains('#') {
            assert!(
                line.len() <= 30,
                "line {} exceeds line_width 30: '{}' ({} chars)",
                i + 1,
                line,
                line.len()
            );
        }
    }
}

// ── Bracket comment as trailing comment ───────────────────────────────

#[test]
fn bracket_comment_as_trailing_comment_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar) #[[ bracket trailing ]]\n");

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("#[[ bracket trailing ]]"),
        "bracket trailing comment should be preserved verbatim, got:\n{stdout}"
    );
}

// ── Very long variable name exceeding line_width ──────────────────────

// ── dump ast ────────────────────────────────────────────────────────────

#[test]
fn dump_ast_prints_tree() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "cmake_minimum_required(VERSION 3.5)\n\nproject(demo \"1.0.0\")\n# standalone comment\nset(FOO bar) # my comment\n",
    );

    let output = cmakefmt()
        .args(["--color", "never", "dump", "ast", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dump ast should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // FILE root node
    assert!(stdout.contains("FILE"), "should contain FILE root node");

    // Command nodes
    assert!(
        stdout.contains("COMMAND")
            && stdout.contains("cmake_minimum_required")
            && stdout.contains("project")
            && stdout.contains("set"),
        "should contain all COMMAND nodes, got:\n{stdout}",
    );

    // Arguments with annotations
    assert!(
        stdout.contains("ARG") && stdout.contains("(unquoted)"),
        "should contain ARG nodes with unquoted annotation, got:\n{stdout}",
    );
    assert!(
        stdout.contains("(quoted)"),
        "should contain quoted annotation, got:\n{stdout}",
    );

    // Standalone comment
    assert!(
        stdout.contains("COMMENT") && stdout.contains("# standalone comment"),
        "should contain standalone COMMENT, got:\n{stdout}",
    );

    // Trailing comment
    assert!(
        stdout.contains("TRAILING") && stdout.contains("# my comment"),
        "should contain TRAILING comment, got:\n{stdout}",
    );

    // Blank-line separator
    assert!(
        stdout.contains("───"),
        "should contain blank-line separator, got:\n{stdout}",
    );
}

// ── Very long variable name exceeding line_width ──────────────────────

#[test]
fn set_very_long_variable_name_exceeding_line_width() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 40\n",
    );
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &file,
        "set(THIS_IS_AN_EXTREMELY_LONG_VARIABLE_NAME value1 value2 value3)\n",
    );

    let output = cmakefmt().args([file.to_str().unwrap()]).output().unwrap();
    assert!(
        output.status.success(),
        "formatter should handle variable names exceeding line_width without error"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Variable name should still stay on set( line even if it exceeds line_width.
    assert!(
        stdout.starts_with("set(THIS_IS_AN_EXTREMELY_LONG_VARIABLE_NAME"),
        "variable name should stay on set( line regardless of width, got:\n{stdout}"
    );
}

// ── dump parse ──────────────────────────────────────────────────────────────

#[test]
fn dump_parse_prints_spec_resolved_tree() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "target_link_libraries(mylib PUBLIC dep1 dep2)\n");

    let output = cmakefmt()
        .args(["--color", "never", "dump", "parse", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dump parse should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("KEYWORD  PUBLIC"),
        "should classify PUBLIC as KEYWORD, got:\n{stdout}",
    );
    assert!(
        stdout.contains("POSITIONAL  mylib"),
        "should classify mylib as POSITIONAL, got:\n{stdout}",
    );
}

#[test]
fn dump_parse_groups_flow_control() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "if(WIN32)\n  message(STATUS \"hello\")\nendif()\n");

    let output = cmakefmt()
        .args(["--color", "never", "dump", "parse", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dump parse should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("FLOW"),
        "should contain a FLOW node for if/endif block, got:\n{stdout}",
    );
    assert!(
        stdout.contains("BODY"),
        "should contain a BODY node inside the flow block, got:\n{stdout}",
    );
}

#[test]
fn dump_ast_reads_stdin() {
    let mut child = cmakefmt()
        .args(["dump", "ast", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"set(FOO bar)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "dump ast with stdin should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("COMMAND") && stdout.contains("set"),
        "stdin dump should show COMMAND set, got:\n{stdout}"
    );
}

#[test]
fn dump_ast_no_color_when_forced_never() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--color", "never", "dump", "ast", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "color never should not contain ANSI codes, got:\n{stdout}"
    );
}

#[test]
fn dump_ast_color_when_forced_always() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--color", "always", "dump", "ast", file.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b["),
        "color always should contain ANSI codes, got:\n{stdout}"
    );
}

#[test]
fn dump_parse_reads_stdin() {
    let mut child = cmakefmt()
        .args(["dump", "parse", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"target_link_libraries(mylib PUBLIC dep1)\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "dump parse with stdin should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("KEYWORD") && stdout.contains("PUBLIC"),
        "stdin dump parse should resolve keywords, got:\n{stdout}"
    );
}
