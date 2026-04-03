use std::io::Write;
use std::process::Command;

fn cmakefmt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
}

fn write_file(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
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
fn explicit_non_cmake_file_is_formatted() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("toolchain.txt");
    write_file(&file, "set(  FOO  bar )\n");

    let output = cmakefmt().arg(file.to_str().unwrap()).output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "set(FOO bar)\n");
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
        .args(["--list-files"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CMakeLists.txt"));
    assert!(stdout.contains("CompilerWarnings.cmake"));
    assert!(!stdout.contains("example.txt"));
}

#[test]
fn directory_input_discovers_only_cmake_files() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("cmake/toolchain.cmake.in");
    let ignored = dir.path().join("cmake/ignore.txt");

    write_file(&nested, "set(  FOO  bar )\n");
    write_file(&ignored, "set(  NOPE  value )\n");

    let output = cmakefmt()
        .args(["--list-files", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
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
            "--list-files",
            "--path-regex",
            "Keep",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("KeepThis.cmake"));
    assert!(!stdout.contains("SkipThis.cmake"));
}

#[test]
fn list_files_reports_only_changed_targets() {
    let dir = tempfile::tempdir().unwrap();
    let changed = dir.path().join("changed.cmake");
    let clean = dir.path().join("clean.cmake");

    write_file(&changed, "set(  FOO  bar )\n");
    write_file(&clean, "set(FOO bar)\n");

    let output = cmakefmt()
        .args(["--list-files", dir.path().to_str().unwrap()])
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
        subdir.join(".cmakefmt.toml"),
        "[style]\nkeyword_case = \"lower\"\n",
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
    let config_path = dir.path().join("custom.toml");
    std::fs::write(
        &config_path,
        r#"
[format]
line_width = 30

[commands.my_custom_command]
pargs = 1

[commands.my_custom_command.kwargs.SOURCES]
nargs = "+"

[commands.my_custom_command.kwargs.LIBRARIES]
nargs = "+"
"#,
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
    let output = cmakefmt().arg("--print-default-config").output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Default cmakefmt configuration."));
    assert!(stdout.contains("[format]"));
    assert!(stdout.contains("line_width = 80"));
    assert!(stdout.contains("# use_tabs = true"));
    assert!(stdout.contains("[markup]"));
    assert!(stdout.contains("# [per_command.message]"));
    assert!(stdout.contains("# [commands.my_custom_command]"));
}

#[test]
fn debug_mode_reports_config_and_barriers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(
        dir.path().join(".cmakefmt.toml"),
        "[format]\nline_width = 40\n",
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
    assert!(stderr.contains("formatter: disabled formatting"));
    assert!(stderr.contains("formatter: enabled formatting"));
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

// ── Version ─────────────────────────────────────────────────────────────────

#[test]
fn version_flag() {
    let output = cmakefmt().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cmakefmt"));
}

#[test]
fn help_mentions_config_discovery_and_primary_flags() {
    let output = cmakefmt().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parse CMake listfiles and format them nicely."));
    assert!(stdout.contains(".cmakefmt.toml"));
    assert!(stdout.contains("--colour <COLOUR>"));
    assert!(stdout.contains("--print-default-config"));
    assert!(stdout.contains("--in-place"));
    assert!(stdout.contains("--config-file <PATH>"));
    assert!(stdout.contains("--convert-legacy-config <PATH>"));
    assert!(stdout.contains("--list-files"));
    assert!(stdout.contains("--path-regex <REGEX>"));
    assert!(stdout.contains("formatting stays single-threaded"));
}
