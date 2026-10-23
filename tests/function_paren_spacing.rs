// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::process::Command;

use cmakefmt::{
    config::{convert_legacy_config_files, file::DumpConfigFormat},
    format_source,
    semantic::semantic_equivalent,
    Config, DangleAlign, WrapAfterFirstArgAlign,
};

#[test]
fn ordinary_calls_can_have_spaces_independently_of_control_and_definitions() {
    let source = "project(foo VERSION ${bar})\nmy_helper(first second)\ncustom_empty()\nif(TRUE)\nfunction(my_func ARG)\nmessage(STATUS \"ready\")\nendfunction()\nendif()\n";
    let config = Config {
        separate_fn_name_with_space: true,
        ..Config::default()
    };
    let formatted = format_source(source, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    project (foo VERSION ${bar})
    my_helper (first second)
    custom_empty ()
    if(TRUE)
      function(my_func ARG)
        message (STATUS "ready")
      endfunction()
    endif()
    "#);
    assert_eq!(formatted, format_source(&formatted, &config).unwrap());
    assert!(semantic_equivalent(source, &formatted));
    let default = format_source(&formatted, &Config::default()).unwrap();
    assert!(default.starts_with("project(foo VERSION ${bar})\n"));
    assert!(default.contains("custom_empty()\n"));
}

#[test]
fn spacing_counts_towards_wrapping_and_paren_alignment() {
    let source = "set(SOURCES first.cpp second.cpp third.cpp fourth.cpp)\n";
    let config = Config {
        separate_fn_name_with_space: true,
        line_width: 30,
        always_wrap: vec!["set".into()],
        dangle_parens: true,
        dangle_align: DangleAlign::Open,
        ..Config::default()
    };
    let formatted = format_source(source, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    set (SOURCES
         first.cpp second.cpp
         third.cpp fourth.cpp
        )
    ");
    assert!(formatted.lines().all(|line| line.len() <= 30));
    assert_eq!(formatted, format_source(&formatted, &config).unwrap());
}

#[test]
fn space_can_push_inline_call_over_width() {
    let source = "message(hello)\n";
    let mut config = Config {
        line_width: 14,
        ..Config::default()
    };
    assert_eq!(format_source(source, &config).unwrap(), source);
    config.separate_fn_name_with_space = true;
    let formatted = format_source(source, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    message (
      hello)
    ");
    assert!(formatted.lines().all(|line| line.len() <= 14));
    assert_eq!(formatted, format_source(&formatted, &config).unwrap());
}

#[test]
fn spacing_is_stable_across_layouts_and_preserves_comments() {
    let source = "if(ON)\ntarget_compile_definitions(mylib # target\n PRIVATE # visibility\n USING_DLL COMMON_USING_DLL EXTRA_DEFINE)\nset(SOURCES first.cpp second.cpp third.cpp fourth.cpp)\nmy_helper(first second third fourth fifth sixth seventh)\nendif()\n";
    for separate_fn_name_with_space in [false, true] {
        for line_width in [35, 80] {
            for use_tabchars in [false, true] {
                for align in [
                    WrapAfterFirstArgAlign::UnderFirstArg,
                    WrapAfterFirstArgAlign::DefaultIndent,
                ] {
                    for dangle in [
                        None,
                        Some(DangleAlign::Prefix),
                        Some(DangleAlign::Open),
                        Some(DangleAlign::Close),
                    ] {
                        let config = Config {
                            separate_fn_name_with_space,
                            line_width,
                            use_tabchars,
                            wrap_after_first_arg: true,
                            wrap_after_first_arg_align: align,
                            dangle_parens: dangle.is_some(),
                            dangle_align: dangle.unwrap_or_default(),
                            ..Config::default()
                        };
                        let formatted = format_source(source, &config).unwrap();
                        assert_eq!(
                            formatted,
                            format_source(&formatted, &config).unwrap(),
                            "{config:?}"
                        );
                        assert!(
                            semantic_equivalent(source, &formatted),
                            "{config:?}\n{formatted}"
                        );
                        assert!(formatted.contains("# target"));
                        assert!(formatted.contains("# visibility"));
                    }
                }
            }
        }
    }
}

#[test]
fn yaml_and_toml_settings_round_trip_and_cli_overrides_them() {
    for (extension, contents) in [
        ("yaml", "format:\n  separate_fn_name_with_space: true\n"),
        ("toml", "[format]\nseparate_fn_name_with_space = true\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!(".cmakefmt.{extension}"));
        std::fs::write(&path, contents).unwrap();
        let config = Config::from_file(&path).unwrap();
        assert!(config.separate_fn_name_with_space);
        let source = dir.path().join("CMakeLists.txt");
        std::fs::write(&source, "project(foo)\n").unwrap();
        for (value, expected) in [("true", "project (foo)\n"), ("false", "project(foo)\n")] {
            let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
                .args([
                    "--config-file",
                    path.to_str().unwrap(),
                    "--separate-fn-name-with-space",
                    value,
                ])
                .arg(&source)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
        }
        for format in [DumpConfigFormat::Yaml, DumpConfigFormat::Toml] {
            let rendered =
                cmakefmt::config::file::render_effective_config(&config, format).unwrap();
            let rendered_path = dir.path().join(match format {
                DumpConfigFormat::Yaml => "dump.yaml",
                DumpConfigFormat::Toml => "dump.toml",
            });
            std::fs::write(&rendered_path, rendered).unwrap();
            assert!(
                Config::from_file(&rendered_path)
                    .unwrap()
                    .separate_fn_name_with_space
            );
        }
        std::fs::write(&path, contents.replace("true", "invalid")).unwrap();
        assert!(Config::from_file(&path).is_err());
    }
}

#[test]
fn cli_flags_match_cmake_format_function_and_control_categories() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".cmakefmt.yaml");
    std::fs::write(
        &config_path,
        "format:\n  separate_fn_name_with_space: true\n  separate_ctrl_name_with_space: false\n",
    )
    .unwrap();
    let source = dir.path().join("CMakeLists.txt");
    std::fs::write(&source, "project(foo)\nif(ON)\n  function(helper)\n    custom_call()\n    return()\n  endfunction()\n  macro(helper_macro)\n    custom_call()\n  endmacro()\nendif()\n").unwrap();
    for function_spacing in [false, true] {
        for control_spacing in [false, true] {
            let call = if function_spacing { " " } else { "" };
            let flow = if control_spacing { " " } else { "" };
            let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
                .arg("--config-file")
                .arg(&config_path)
                .args([
                    "--separate-fn-name-with-space",
                    &function_spacing.to_string(),
                ])
                .args([
                    "--separate-ctrl-name-with-space",
                    &control_spacing.to_string(),
                ])
                .arg(&source)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let expected = format!("project{call}(foo)\nif{flow}(ON)\n  function{flow}(helper)\n    custom_call{call}()\n    return{call}()\n  endfunction{flow}()\n  macro{flow}(helper_macro)\n    custom_call{call}()\n  endmacro{flow}()\nendif{flow}()\n");
            assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
        }
    }
    let help = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("--separate-fn-name-with-space"));
    assert!(help.contains("--separate-ctrl-name-with-space"));
}

#[test]
fn boolean_config_flags_accept_bare_and_explicit_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("CMakeLists.txt");
    let config = dir.path().join(".cmakefmt.yaml");
    for (flag, key, source, enabled, disabled) in [
        (
            "--separate-fn-name-with-space",
            "separate_fn_name_with_space",
            "project(foo)\n",
            "project (foo)\n",
            "project(foo)\n",
        ),
        (
            "--separate-ctrl-name-with-space",
            "separate_ctrl_name_with_space",
            "if(ON)\nendif()\n",
            "if (ON)\nendif ()\n",
            "if(ON)\nendif()\n",
        ),
        (
            "--dangle-parens",
            "dangle_parens",
            "set(VAR first second third fourth fifth sixth)\n",
            "set(VAR\n    first second third\n    fourth fifth sixth\n)\n",
            "set(VAR\n    first second third\n    fourth fifth sixth)\n",
        ),
    ] {
        std::fs::write(&path, source).unwrap();
        for configured in [false, true] {
            std::fs::write(
                &config,
                format!("format:\n  line_width: 25\n  {key}: {configured}\n"),
            )
            .unwrap();
            for (args, expected) in [
                (vec![], if configured { enabled } else { disabled }),
                (vec![flag], enabled),
                (vec![flag, "true"], enabled),
                (vec![flag, "false"], disabled),
            ] {
                let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
                    .arg("--config-file")
                    .arg(&config)
                    .arg(&path)
                    .args(&args)
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{args:?}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(
                    String::from_utf8(result.stdout).unwrap(),
                    expected,
                    "{args:?}, configured={configured}"
                );
            }
        }
    }
    // A following option must leave the previous flag's optional value omitted.
    std::fs::write(&path, "if(ON)\nproject(foo)\nendif()\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
        .arg("--no-config")
        .arg(&path)
        .args([
            "--separate-fn-name-with-space",
            "--separate-ctrl-name-with-space",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "if (ON)\n  project (foo)\nendif ()\n"
    );
}

#[test]
fn legacy_function_and_control_spacing_convert_to_the_right_commands() {
    for function_spacing in [false, true] {
        for control_spacing in [false, true] {
            let python_bool = |value| if value { "True" } else { "False" };
            for (extension, contents) in [
                ("py", format!("with section('format'):\n  separate_fn_name_with_space = {}\n  separate_ctrl_name_with_space = {}\n", python_bool(function_spacing), python_bool(control_spacing))),
                ("yaml", format!("format:\n  separate_fn_name_with_space: {function_spacing}\n  separate_ctrl_name_with_space: {control_spacing}\n")),
                ("json", serde_json::json!({"format": {
                    "separate_fn_name_with_space": function_spacing,
                    "separate_ctrl_name_with_space": control_spacing,
                }}).to_string()),
            ] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join(format!(".cmake-format.{extension}"));
                std::fs::write(&path, contents).unwrap();
                for format in [DumpConfigFormat::Yaml, DumpConfigFormat::Toml] {
                    let converted = convert_legacy_config_files(std::slice::from_ref(&path), format).unwrap();
                    let output_path = dir.path().join(match format {
                        DumpConfigFormat::Yaml => "converted.yaml",
                        DumpConfigFormat::Toml => "converted.toml",
                    });
                    std::fs::write(&output_path, converted).unwrap();
                    let config = Config::from_file(&output_path).unwrap();
                    // cmake-format classifies both function and macro definitions
                    // as control flow, independently of ordinary call spacing.
                    let call = if function_spacing { " " } else { "" };
                    let flow = if control_spacing { " " } else { "" };
                    let source = format!("project{call}(foo VERSION ${{bar}})\nif{flow}(TRUE)\n  function{flow}(helper)\n    message{call}(STATUS ready)\n    return{call}()\n  endfunction{flow}()\n  macro{flow}(my_macro)\n    custom_call{call}()\n  endmacro{flow}()\nendif{flow}()\nforeach{flow}(item IN ITEMS one)\n  break{call}()\n  continue{call}()\nendforeach{flow}()\n");
                    assert_eq!(format_source(&source, &config).unwrap(), source);
                }
            }
        }
    }
}

#[test]
fn released_definition_key_has_a_working_migration_hint() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".cmakefmt.yaml");
    let source = dir.path().join("CMakeLists.txt");
    std::fs::write(&source, "project(foo)\n").unwrap();
    std::fs::write(&config, "format:\n  space_before_definition_paren: true\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
        .arg("--config-file")
        .arg(&config)
        .arg(&source)
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("use `separate_ctrl_name_with_space` or run `cmakefmt config convert`"),
        "{stderr}"
    );
    let converted =
        convert_legacy_config_files(std::slice::from_ref(&config), DumpConfigFormat::Yaml).unwrap();
    let converted_path = dir.path().join("converted.yaml");
    std::fs::write(&converted_path, &converted).unwrap();
    let loaded = Config::from_file(&converted_path).unwrap();
    assert!(loaded.for_command("function").space_before_paren());
    assert!(converted.contains("this also affects other structural commands"));
}

#[test]
fn released_control_key_is_an_input_alias_but_dumps_use_canonical_names() {
    for (extension, contents) in [
        ("yaml", "format:\n  space_before_control_paren: true\n"),
        ("toml", "[format]\nspace_before_control_paren = true\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!(".cmakefmt.{extension}"));
        std::fs::write(&path, contents).unwrap();
        let config = Config::from_file(&path).unwrap();
        assert!(config.separate_ctrl_name_with_space);
        assert!(!config.separate_fn_name_with_space);
        assert!(config.for_command("function").space_before_paren());
        let rendered =
            cmakefmt::config::render_effective_config(&config, DumpConfigFormat::Yaml).unwrap();
        let rendered: serde_json::Value = serde_saphyr::from_str(&rendered).unwrap();
        assert_eq!(rendered["format"]["separate_ctrl_name_with_space"], true);
        assert_eq!(rendered["format"]["separate_fn_name_with_space"], false);
        assert!(rendered["format"]
            .get("space_before_control_paren")
            .is_none());
    }
}

#[test]
fn config_output_and_schemas_expose_exactly_two_spacing_controls() {
    let result = Command::new(env!("CARGO_BIN_EXE_cmakefmt"))
        .args(["--no-config", "config", "show"])
        .output()
        .unwrap();
    assert!(result.status.success());
    let shown: serde_json::Value =
        serde_saphyr::from_str(std::str::from_utf8(&result.stdout).unwrap()).unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(&cmakefmt::config::generate_json_schema()).unwrap();
    assert_eq!(
        schema["$defs"]["FormatSection"]["properties"]["space_before_control_paren"]["deprecated"],
        true
    );
    for properties in [
        shown["format"].as_object().unwrap(),
        schema["$defs"]["FormatSection"]["properties"]
            .as_object()
            .unwrap(),
    ] {
        let mut keys: Vec<_> = properties
            .keys()
            .filter(|key| key.starts_with("separate_"))
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "separate_ctrl_name_with_space",
                "separate_fn_name_with_space"
            ]
        );
    }
    let dprint_schema: serde_json::Value =
        serde_json::from_str(include_str!("../packaging/dprint-cmakefmt.schema.json")).unwrap();
    let mut keys: Vec<_> = dprint_schema["properties"]["cmakefmt"]["properties"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| key.starts_with("separate"))
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["separateCtrlNameWithSpace", "separateFnNameWithSpace"]
    );
}
