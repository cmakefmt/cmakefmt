use std::io::Write;
use std::process::Command;

fn cmfmt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cmfmt"))
}

// ── Basic formatting ────────────────────────────────────────────────────────

#[test]
fn formats_file_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "cmake_minimum_required( VERSION   3.20 )\n").unwrap();

    let output = cmfmt().arg(file.to_str().unwrap()).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cmake_minimum_required(VERSION 3.20)\n"
    );
}

#[test]
fn reads_stdin_with_dash() {
    let mut child = cmfmt()
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

// ── In-place formatting ─────────────────────────────────────────────────────

#[test]
fn in_place_modifies_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("CMakeLists.txt");
    std::fs::write(&file, "set(  FOO  bar )\n").unwrap();

    let output = cmfmt()
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
    cmfmt()
        .args(["-i", file.to_str().unwrap()])
        .output()
        .unwrap();
    let first = std::fs::read_to_string(&file).unwrap();

    // Format again
    cmfmt()
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

    let output = cmfmt()
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

    let output = cmfmt()
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

    cmfmt()
        .args(["--check", file.to_str().unwrap()])
        .output()
        .unwrap();

    let contents = std::fs::read_to_string(&file).unwrap();
    assert_eq!(contents, original);
}

// ── Error handling ──────────────────────────────────────────────────────────

#[test]
fn nonexistent_file_returns_exit_2() {
    let output = cmfmt().arg("/nonexistent/file.cmake").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
}

// ── CLI flag overrides ──────────────────────────────────────────────────────

#[test]
fn line_width_override() {
    let mut child = cmfmt()
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
fn command_case_override() {
    let mut child = cmfmt()
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

    let output = cmfmt().args(&args).output().unwrap();
    assert!(output.status.success());

    for (i, path) in paths.iter().enumerate() {
        let contents = std::fs::read_to_string(path).unwrap();
        assert_eq!(contents, format!("set(VAR_{i} value)\n"));
    }
}

// ── Config file ─────────────────────────────────────────────────────────────

#[test]
fn explicit_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "[style]\ncommand_case = \"upper\"\n").unwrap();

    let mut child = cmfmt()
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("CMAKE_MINIMUM_REQUIRED("));
}

// ── Version ─────────────────────────────────────────────────────────────────

#[test]
fn version_flag() {
    let output = cmfmt().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cmfmt"));
}
