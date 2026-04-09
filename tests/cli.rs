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
        "style:\n  command_case: upper\n",
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
    std::fs::write(&config_path, "[style]\ncommand_case = \"upper\"\n").unwrap();

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
    std::fs::write(&config_path, "[style]\ncommand_case = \"upper\"\n").unwrap();

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
    std::fs::write(&first, "[style]\ncommand_case = \"upper\"\n").unwrap();
    std::fs::write(&second, "[style]\nkeyword_case = \"lower\"\n").unwrap();

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
    std::fs::write(&config_path, "[style]\ncommand_case = \"upper\"\n").unwrap();

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
        .args(["--convert-legacy-config", legacy.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Converted from legacy cmake-format configuration."));
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 100"));
    assert!(stdout.contains("style:"));
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
            "--convert-legacy-config",
            legacy.to_str().unwrap(),
            "--convert-legacy-config-format",
            "toml",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Converted from legacy cmake-format configuration."));
    assert!(stdout.contains("[format]"));
    assert!(stdout.contains("line_width = 100"));
    assert!(stdout.contains("[style]"));
    assert!(stdout.contains("command_case = \"lower\""));
}

#[test]
fn convert_config_conflicts_with_input_paths() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("cmake-format.json");
    std::fs::write(&legacy, "{}").unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([
            "--convert-legacy-config",
            legacy.to_str().unwrap(),
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not accept formatting input paths")
    );
}

#[test]
fn discovered_config_uses_nearest_file_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.toml"),
        "[style]\ncommand_case = \"upper\"\n",
    )
    .unwrap();

    let subdir = dir.path().join("nested");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(
        subdir.join(".cmakefmt.yaml"),
        "style:\n  keyword_case: lower\n",
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
        "format:\n  line_width: 30\ncommands:\n  my_custom_command:\n    pargs: 1\n    kwargs:\n      SOURCES:\n        nargs: \"+\"\n      LIBRARIES:\n        nargs: \"+\"\n",
    )
    .unwrap();

    let input = dir.path().join("input.cmake");
    write_file(
        &input,
        "my_custom_command(target SOURCES a.cpp b.cpp c.cpp LIBRARIES foo bar)\n",
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
    assert!(stdout.contains("my_custom_command("));
    assert!(stdout.contains("\n  SOURCES a.cpp b.cpp c.cpp\n"));
    assert!(stdout.contains("\n  LIBRARIES foo bar)"));
}

#[test]
fn dump_config_prints_template() {
    let output = cmakefmt().arg("--dump-config").output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Default cmakefmt configuration."));
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 80"));
    assert!(stdout.contains("# use_tabs: true"));
    assert!(stdout.contains("markup:"));
    assert!(stdout.contains("# per_command_overrides:"));
    assert!(stdout.contains("# commands:"));
    assert!(stdout.contains("#   my_custom_command:"));
}

#[test]
fn dump_config_toml_prints_template() {
    let output = cmakefmt().args(["--dump-config", "toml"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Default cmakefmt configuration."));
    assert!(stdout.contains("[format]"));
    assert!(stdout.contains("line_width = 80"));
    assert!(stdout.contains("# use_tabs = true"));
    assert!(stdout.contains("[markup]"));
    assert!(stdout.contains("# [per_command_overrides.my_custom_command]"));
    assert!(stdout.contains("# [commands.my_custom_command]"));
}

#[test]
fn show_config_prints_effective_yaml_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(
        &dir.path().join(".cmakefmt.yaml"),
        "format:\n  line_width: 99\nstyle:\n  command_case: upper\n",
    );
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--show-config", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format:"));
    assert!(stdout.contains("line_width: 99"));
    assert!(stdout.contains("style:"));
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
            "--show-config",
            "--line-width",
            "120",
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
        "[style]\ncommand_case = \"upper\"\n",
    );
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    write_file(
        &nested.join(".cmakefmt.yaml"),
        "style:\n  command_case: lower\n",
    );
    let file = nested.join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--show-config-path", file.to_str().unwrap()])
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
    write_file(&config, "style:\n  command_case: upper\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args([file.to_str().unwrap(), "--show-config-path"])
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
    write_file(&config, "style:\n  command_case: upper\n");
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--find-config-path", file.to_str().unwrap()])
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
        .args(["--no-config", "--show-config", file.to_str().unwrap()])
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
            "--explain-config",
            file.to_str().unwrap(),
            "--line-width",
            "120",
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
        .args([file.to_str().unwrap(), "--explain-config"])
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
        .arg("--explain-config")
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
        .args(["--show-config", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("config introspection expects exactly one explicit path"));
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
fn progress_bar_requires_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    write_file(&file, "set(  FOO  value )\n");

    let output = cmakefmt()
        .args(["--progress-bar", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--in-place"));
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
    let output = cmakefmt()
        .args(["--generate-completion", "bash"])
        .output()
        .unwrap();

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
    assert!(stdout.contains("--colour <COLOUR>"));
    assert!(stdout.contains("--dump-config [<FORMAT>]"));
    assert!(stdout.contains("--generate-completion <SHELL>"));
    assert!(stdout.contains("--generate-man-page"));
    assert!(stdout.contains("--required-version <VERSION>"));
    assert!(stdout.contains("--verify"));
    assert!(stdout.contains("--fast"));
    assert!(stdout.contains("--require-pragma"));
    assert!(stdout.contains("--show-config[=<FORMAT>]"));
    assert!(stdout.contains("--show-config-path"));
    assert!(stdout.contains("--find-config-path"));
    assert!(stdout.contains("--explain-config"));
    assert!(stdout.contains("--in-place"));
    assert!(stdout.contains("-c, --config-file <PATH>"));
    assert!(stdout.contains("--config-file <PATH>"));
    assert!(stdout.contains("--no-config"));
    assert!(stdout.contains("--convert-legacy-config <PATH>"));
    assert!(stdout.contains("-l, --line-width <LINE_WIDTH>"));
    assert!(stdout.contains("--list-changed-files"));
    assert!(stdout.contains("--list-input-files"));
    assert!(stdout.contains("--path-regex <REGEX>"));
    assert!(stdout.contains("--ignore-path <PATH>"));
    assert!(stdout.contains("--no-gitignore"));
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
    assert!(stdout.contains("formatting stays single-threaded"));
}

// ── --dump-schema ───────────────────────────────────────────────────────────

#[test]
fn dump_schema_exits_zero() {
    let output = cmakefmt().arg("--dump-schema").output().unwrap();
    assert!(output.status.success());
}

#[test]
fn dump_schema_prints_valid_json() {
    let output = cmakefmt().arg("--dump-schema").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value =
        serde_json::from_str(&stdout).expect("--dump-schema output should be valid JSON");
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
    let output = cmakefmt().arg("--dump-schema").output().unwrap();
    assert!(
        output.stdout.ends_with(b"\n"),
        "--dump-schema output should end with a newline"
    );
}

#[test]
fn dump_schema_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--dump-schema"));
}

// ── init subcommand ────────────────────────────────────────────────────────

#[test]
fn init_creates_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let output = cmakefmt()
        .args(["init"])
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
    std::fs::write(dir.path().join(".cmakefmt.yaml"), "line_width: 100\n").unwrap();

    let output = cmakefmt()
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    // Original file should be unchanged
    let content = std::fs::read_to_string(dir.path().join(".cmakefmt.yaml")).unwrap();
    assert_eq!(content, "line_width: 100\n");
}

// ── --lsp ───────────────────────────────────────────────────────────────────

#[test]
fn lsp_appears_in_help() {
    let output = cmakefmt().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--lsp"));
}
