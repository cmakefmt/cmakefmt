// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;
use std::path::PathBuf;

use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{format_source, CaseStyle, Config, DangleAlign, PerCommandConfig};
use cmakefmt::{formatter, parser};

// --- Parser edge-case / coverage tests ---

#[test]
fn empty_input_formats_to_empty() {
    assert_eq!(format_source("", &Config::default()).unwrap(), "");
}

#[test]
fn command_with_no_arguments() {
    let src = "message()\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"message()");
}

#[test]
fn string_with_special_characters() {
    let src = "set(VAR \"hello !@#$%^&*()\")\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"set(VAR "hello !@#$%^&*()")"#);
}

#[test]
fn deeply_nested_blocks() {
    let src = "if(A)\n  if(B)\n    if(C)\n      message(deep)\n    endif()\n  endif()\nendif()\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    // Should preserve nesting structure
    assert!(formatted.contains("message(deep)"));
    assert_eq!(
        formatted.lines().filter(|l| l.contains("endif()")).count(),
        3
    );
}

#[test]
fn cmake_minimum_required_with_fatal_error() {
    let src = "cmake_minimum_required(VERSION 3.20 FATAL_ERROR)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"cmake_minimum_required(VERSION 3.20 FATAL_ERROR)");
}

#[test]
fn file_with_only_comments_parses_successfully() {
    let src = "# first comment\n# second comment\n# third comment\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    # first comment
    # second comment
    # third comment
    ");
}

#[test]
fn bracket_argument_with_special_chars() {
    let src = "set(VAR [==[value with special chars: !@#$%^&*()]==])\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    // Bracket arguments force multiline layout
    assert!(formatted.contains("VAR"));
    assert!(formatted.contains("[==[value with special chars: !@#$%^&*()]==]"));
}

// --- formatter/node.rs coverage tests ---

#[test]
fn max_rows_cmdline_one_forces_vertical() {
    // max_rows_cmdline=1: hanging wrap that produces more than 1 row is rejected
    // → falls back to vertical layout.
    // Use a command that doesn't fit inline but would normally use hanging wrap.
    // Vertical layout opens the paren on the first line alone ("set(\n").
    // Hanging wrap would put the first token right after "set(".
    let src = "set(AVERYLONG_VAR_NAME_HERE B C D E F)\n";
    let config = Config {
        max_rows_cmdline: 1,
        line_width: 35,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let first_line = formatted.lines().next().unwrap();
    // With vertical layout the opening line is just "set(" — no args follow on
    // the same line. With hanging wrap it would be "set(AVERYLONG_VAR_NAME_HERE".
    assert_eq!(
        first_line, "set(",
        "expected vertical layout (first line = 'set('), got:\n{formatted}"
    );
}

#[test]
fn max_subgroups_hwrap_zero_forces_vertical() {
    // With a narrow line width the command doesn't fit inline.
    // max_subgroups_hwrap=0 is honoured as a threshold that forces vertical
    // when the hanging path is already unavailable for multi-section commands.
    let src = "target_link_libraries(mylib PUBLIC dep1 dep2 dep3 PRIVATE helper1 helper2)\n";
    let config = Config {
        max_subgroups_hwrap: 0,
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.lines().count() > 1,
        "expected multiline output, got:\n{formatted}"
    );
}

#[test]
fn dangle_align_close() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux corge grault garply)\n";
    let config = Config {
        line_width: 40,
        dangle_parens: true,
        dangle_align: DangleAlign::Close,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let last = formatted.lines().last().unwrap();
    // Close alignment: ) at current indentation level (0 indent = column 0)
    assert_eq!(
        last.trim_start(),
        ")",
        "expected dangling ) with Close alignment, got last line: {last:?}"
    );
}

#[test]
fn space_before_control_paren_formats_if() {
    let src = "if(TRUE)\nmessage(ok)\nendif()\n";
    let config = Config {
        separate_ctrl_name_with_space: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("if (TRUE)"),
        "expected 'if (TRUE)' in output, got:\n{formatted}"
    );
    assert!(
        formatted.contains("endif ()"),
        "expected 'endif ()' in output, got:\n{formatted}"
    );
}

#[test]
fn space_before_definition_paren_formats_function() {
    let src = "function(my_func ARG1)\nmessage(${ARG1})\nendfunction()\n";
    let config = Config {
        separate_fn_name_with_space: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("function (my_func"),
        "expected 'function (my_func' in output, got:\n{formatted}"
    );
    assert!(
        formatted.contains("endfunction ()"),
        "expected 'endfunction ()' in output, got:\n{formatted}"
    );
}

#[test]
fn explicit_trailing_pattern_keeps_comment_inline() {
    // The default explicit_trailing_pattern is "#<" — a comment matching it
    // stays on the same line as the preceding argument.
    // Use max_pargs_hwrap=1 to force write_vertical_arguments for the PRIVATE
    // section, which is where the explicit_trailing_pattern logic lives.
    let src = "target_sources(foo\n  PRIVATE\n    a.cc #< keep\n    b.cc\n)\n";
    let config = Config {
        max_pargs_hwrap: 1,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let has_inline = formatted
        .lines()
        .any(|l| l.contains("a.cc") && l.contains("#<"));
    assert!(
        has_inline,
        "explicit trailing comment should stay inline with preceding arg, got:\n{formatted}"
    );
}

#[test]
fn canonicalize_hashrulers_false_preserves_ruler() {
    let src = "# -----  uneven ruler  -----\nmessage(ok)\n";
    let config = Config {
        canonicalize_hashrulers: false,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("# -----"),
        "expected ruler to be preserved, got:\n{formatted}"
    );
}

#[test]
fn short_hashruler_not_canonicalized() {
    let src = "# ---\nmessage(ok)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    // default hashruler_min_length=10; "---" (3 chars) is below threshold
    // → preserved as-is, not treated as a ruler
    assert!(
        formatted.contains("# ---"),
        "short ruler should be preserved as-is, got:\n{formatted}"
    );
}

// --- Comment tests ---

#[test]
fn standalone_line_comment() {
    let src = "# this is a comment\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"# this is a comment");
}

#[test]
fn standalone_line_comment_reflows_when_enabled() {
    let src = "# This is a very long comment that should be wrapped when comment reflow is enabled for the formatter.\n";
    let config = Config {
        line_width: 50,
        reflow_comments: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    # This is a very long comment that should be
    # wrapped when comment reflow is enabled for the
    # formatter.
    ");
}

#[test]
fn standalone_empty_comment() {
    let src = "#\nmessage(hello)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    #
    message(hello)
    ");
}

#[test]
fn trailing_comment_on_command() {
    let src = "message(STATUS \"hello\") # trailing\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"message(STATUS "hello") # trailing"#);
}

#[test]
fn comment_before_command() {
    let src = "# set the version\ncmake_minimum_required(VERSION 3.20)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    # set the version
    cmake_minimum_required(VERSION 3.20)
    ");
}

#[test]
fn multiple_consecutive_comments() {
    let src = "# line one\n# line two\n# line three\nmessage(hello)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    # line one
    # line two
    # line three
    message(hello)
    ");
}

#[test]
fn inline_comment_between_arguments() {
    let src = "target_sources(foo\n  PRIVATE\n    a.cc # keep this\n    b.cc\n)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    target_sources(
      foo
      PRIVATE
        a.cc
        # keep this
        b.cc)
    ");
}

#[test]
fn inline_bracket_comment_between_arguments() {
    let src = "message(\"First\" #[[inline comment]] \"Second\")\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    message(
      "First"
      #[[inline comment]]
      "Second")
    "#);
}

#[test]
fn standalone_bracket_comment() {
    let src = "#[[ this is a bracket comment ]]\nmessage(hello)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    #[[ this is a bracket comment ]]
    message(hello)
    ");
}

#[test]
fn comment_separated_by_blank_lines() {
    let src = "message(a)\n\n# between\n\nmessage(b)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"
    message(a)

    # between

    message(b)
    ");
}

#[test]
fn cmakefmt_off_on_preserves_disabled_region() {
    let src = "set(  BEFORE  value )\n# cmakefmt: off\nset(   BROKEN    value )\nthis is not valid cmake\n# cmakefmt: on\nset(  AFTER  value )\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # cmakefmt: off
    set(   BROKEN    value )
    this is not valid cmake
    # cmakefmt: on
    set(AFTER value)
    "#);
}

#[test]
fn cmake_format_off_on_alias_preserves_disabled_region() {
    let src = "set(  BEFORE  value )\n# cmake-format: off\nset(   BROKEN    value )\n# cmake-format: on\nset(  AFTER  value )\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # cmake-format: off
    set(   BROKEN    value )
    # cmake-format: on
    set(AFTER value)
    "#);
}

#[test]
fn fmt_off_on_alias_preserves_disabled_region() {
    let src = "set(  BEFORE  value )\n# fmt: off\nset(   BROKEN    value )\n# fmt: on\nset(  AFTER  value )\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # fmt: off
    set(   BROKEN    value )
    # fmt: on
    set(AFTER value)
    "#);
}

#[test]
fn cmakefmt_off_without_on_preserves_rest_of_file() {
    let src = "set(  BEFORE  value )\n# cmakefmt: off\nset(   BROKEN    value )\nthis is not valid cmake\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # cmakefmt: off
    set(   BROKEN    value )
    this is not valid cmake
    "#);
}

#[test]
fn cmake_format_off_without_on_preserves_rest_of_file() {
    let src = "set(  BEFORE  value )\n# cmake-format: off\nset(   BROKEN    value )\nthis is not valid cmake\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # cmake-format: off
    set(   BROKEN    value )
    this is not valid cmake
    "#);
}

#[test]
fn fmt_off_without_on_preserves_rest_of_file() {
    let src =
        "set(  BEFORE  value )\n# fmt: off\nset(   BROKEN    value )\nthis is not valid cmake\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # fmt: off
    set(   BROKEN    value )
    this is not valid cmake
    "#);
}

#[test]
fn fence_region_preserves_contents_verbatim() {
    let src = "set(  BEFORE  value )\n# ~~~\nset(   BROKEN    value )\nif(  BROKEN )\nmessage( STATUS   \"x\" )\n# ~~~\nset(  AFTER  value )\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set(BEFORE value)
    # ~~~
    set(   BROKEN    value )
    if(  BROKEN )
    message( STATUS   "x" )
    # ~~~
    set(AFTER value)
    "#);
}

// --- Existing tests ---

#[test]
fn wraps_keyword_sections() {
    let src = "target_link_libraries(cmakefmt PUBLIC fmt::fmt another::very_long_dependency_name PRIVATE helper::runtime_support)\n";
    let config = Config {
        line_width: 48,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @"
    target_link_libraries(
      cmakefmt
      PUBLIC
        fmt::fmt another::very_long_dependency_name
      PRIVATE helper::runtime_support)
    ");
}

#[test]
fn discriminated_commands_use_selected_form() {
    let src = "install(TARGETS cmakefmt helper RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib/static)\n";
    let config = Config {
        line_width: 52,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @"
    install(
      TARGETS cmakefmt helper
      RUNTIME
      DESTINATION bin
      LIBRARY
      DESTINATION lib
      ARCHIVE
      DESTINATION lib/static)
    ");
}

#[test]
fn install_targets_recognizes_export_and_includes_sections() {
    let src = "install(TARGETS ${NLOHMANN_JSON_TARGET_NAME} EXPORT ${NLOHMANN_JSON_TARGETS_EXPORT_NAME} INCLUDES DESTINATION ${NLOHMANN_JSON_INCLUDE_INSTALL_DIR})\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    install(
      TARGETS ${NLOHMANN_JSON_TARGET_NAME}
      EXPORT ${NLOHMANN_JSON_TARGETS_EXPORT_NAME}
      INCLUDES DESTINATION ${NLOHMANN_JSON_INCLUDE_INSTALL_DIR})
    "#);
}

#[test]
fn control_blocks_are_indented() {
    let src = "if(FOO)\nmessage(STATUS \"a\")\nif(BAR)\nmessage(STATUS \"b\")\nelse()\nmessage(STATUS \"c\")\nendif()\nendif()\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    if(FOO)
      message(STATUS "a")
      if(BAR)
        message(STATUS "b")
      else()
        message(STATUS "c")
      endif()
    endif()
    "#);
}

#[test]
fn project_keyword_value_with_trailing_comment_stays_inline() {
    let src =
        "project(Catch2 VERSION 3.13.0 # CML version placeholder, don't delete\n  LANGUAGES CXX)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    project(
      Catch2
      VERSION 3.13.0 # CML version placeholder, don't delete
      LANGUAGES CXX)
    "#);
}

#[test]
fn file_strings_uses_recognized_keywords() {
    let src = "file(STRINGS \"include/CLI/Version.hpp\" VERSION_STRING REGEX ${VERSION_REGEX})\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"file(STRINGS "include/CLI/Version.hpp" VERSION_STRING REGEX ${VERSION_REGEX})"#);
}

#[test]
fn cmake_dependent_option_hwraps_in_two_lines() {
    let src = "cmake_dependent_option(CLI11_SANITIZERS \"Download the sanitizers CMake config\" OFF \"NOT CMAKE_VERSION VERSION_LESS 3.15\" OFF)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    cmake_dependent_option(CLI11_SANITIZERS "Download the sanitizers CMake config"
                           OFF "NOT CMAKE_VERSION VERSION_LESS 3.15" OFF)
    "#);
}

#[test]
fn if_condition_breaks_before_boolean_operator() {
    let src = "if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME AND EXISTS \"${CMAKE_CURRENT_SOURCE_DIR}/book\")\n  add_subdirectory(book)\nendif()\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME
       AND EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/book")
      add_subdirectory(book)
    endif()
    "#);
}

#[test]
fn anonymous_sections_go_vertical_after_max_pargs_hwrap() {
    let src = "my_custom_command(one two three four five six seven)\n";
    let config = Config {
        max_pargs_hwrap: 6,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    my_custom_command(
      one
      two
      three
      four
      five
      six
      seven)
    "#);
}

#[test]
fn custom_function_spec_sections_are_honored() {
    let src = "my_custom_command(mylib ARG_TYPES networkaccess networkinformation tls SOURCES a.cpp a.h b.cpp b.h c.cpp c.h d.cpp LIBRARIES spdlog::spdlog HIDDEN_LIBRARIES special_hidden_library)\n";
    let file = parser::parse(src).unwrap();
    let config = Config {
        max_pargs_hwrap: 2,
        ..Config::default()
    };
    let mut registry = CommandRegistry::load().unwrap();
    let overrides = r#"
[commands.my_custom_command]
pargs = 1

[commands.my_custom_command.kwargs.ARG_TYPES]
nargs = "+"

[commands.my_custom_command.kwargs.SOURCES]
nargs = "+"

[commands.my_custom_command.kwargs.LIBRARIES]
nargs = "+"

[commands.my_custom_command.kwargs.HIDDEN_LIBRARIES]
nargs = "+"
"#;
    registry
        .merge_override_str(overrides, PathBuf::from("test-custom-spec.toml"))
        .unwrap();

    let formatted = formatter::format_parsed_file(src, &file, &config, &registry).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    my_custom_command(
      mylib
      ARG_TYPES
        networkaccess
        networkinformation
        tls
      SOURCES
        a.cpp
        a.h
        b.cpp
        b.h
        c.cpp
        c.h
        d.cpp
      LIBRARIES spdlog::spdlog
      HIDDEN_LIBRARIES special_hidden_library)
    "#);
}

// --- Config tests ---

#[test]
fn command_case_upper() {
    let src = "cmake_minimum_required(VERSION 3.20)\n";
    let config = Config {
        command_case: CaseStyle::Upper,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"CMAKE_MINIMUM_REQUIRED(VERSION 3.20)");
}

#[test]
fn command_case_unchanged() {
    let src = "CMAKE_MINIMUM_REQUIRED(VERSION 3.20)\n";
    let config = Config {
        command_case: CaseStyle::Unchanged,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"CMAKE_MINIMUM_REQUIRED(VERSION 3.20)");
}

#[test]
fn keyword_case_lower() {
    let src = "target_link_libraries(foo PUBLIC bar)\n";
    let config = Config {
        keyword_case: CaseStyle::Lower,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"target_link_libraries(foo public bar)");
}

#[test]
fn keyword_case_unchanged() {
    let src = "target_link_libraries(foo Public bar)\n";
    let config = Config {
        keyword_case: CaseStyle::Unchanged,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"target_link_libraries(foo Public bar)");
}

#[test]
fn dangle_parens_true() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux corge grault garply)\n";
    let config = Config {
        line_width: 40,
        dangle_parens: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    target_link_libraries(
      mylib
      PUBLIC
        foo
        bar
        baz
        qux
        quux
        corge
        grault
        garply
    )
    ");
}

#[test]
fn dangle_parens_false() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux corge grault garply)\n";
    let config = Config {
        line_width: 40,
        dangle_parens: false,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    target_link_libraries(
      mylib
      PUBLIC
        foo
        bar
        baz
        qux
        quux
        corge
        grault
        garply)
    ");
}

#[test]
fn separate_ctrl_name_with_space() {
    let src = "if(TRUE)\nendif()\n";
    let config = Config {
        separate_ctrl_name_with_space: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    if (TRUE)
    endif ()
    ");
}

#[test]
fn separate_fn_name_with_space() {
    let src = "function(my_func ARG)\nendfunction()\n";
    let config = Config {
        separate_fn_name_with_space: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    function (my_func ARG)
    endfunction ()
    ");
}

#[test]
fn tab_size_4() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux)\n";
    let config = Config {
        line_width: 40,
        tab_size: 4,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    target_link_libraries(
        mylib
        PUBLIC foo bar baz qux quux)
    ");
}

#[test]
fn use_tabchars() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux corge grault)\n";
    let config = Config {
        line_width: 40,
        use_tabchars: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    // With tabs, indentation uses \t instead of spaces
    assert!(formatted.contains("\t"));
    assert!(formatted.contains("\tmylib"));
}

#[test]
fn max_empty_lines_zero() {
    let src = "message(a)\n\n\n\nmessage(b)\n";
    let config = Config {
        max_empty_lines: 0,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    message(a)
    message(b)
    ");
}

#[test]
fn max_empty_lines_two() {
    let src = "message(a)\n\n\n\n\nmessage(b)\n";
    let config = Config {
        max_empty_lines: 2,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @"
    message(a)


    message(b)
    ");
}

#[test]
fn per_command_override() {
    let src = "SET(VAR value)\nmessage(STATUS \"hello\")\n";
    let mut per_command_overrides = HashMap::new();
    per_command_overrides.insert(
        "set".to_string(),
        PerCommandConfig {
            command_case: Some(CaseStyle::Upper),
            ..Default::default()
        },
    );
    let config = Config {
        command_case: CaseStyle::Lower,
        per_command_overrides,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    SET(VAR value)
    message(STATUS "hello")
    "#);
}

#[test]
fn dangle_align_open() {
    let src = "target_link_libraries(mylib PUBLIC foo bar baz qux quux corge grault garply)\n";
    let config = Config {
        line_width: 40,
        dangle_parens: true,
        dangle_align: DangleAlign::Open,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    // The closing paren should be aligned with the opening paren column (21)
    let lines: Vec<&str> = formatted.lines().collect();
    let last = lines.last().unwrap();
    assert_eq!(last, &format!("{})", " ".repeat(21)));
}

// --- Phase-16 config option tests ---

#[test]
fn disable_returns_source_unchanged() {
    let src = "message(  hello   world  )\n";
    let config = Config {
        disable: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert_eq!(formatted, src);
}

#[test]
fn line_ending_windows_produces_crlf() {
    use cmakefmt::LineEnding;
    let src = "message(hello)\n";
    let config = Config {
        line_ending: LineEnding::Windows,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(formatted.contains("\r\n"), "expected CRLF line ending");
    assert!(!formatted.replace("\r\n", "").contains('\r'), "no stray CR");
}

#[test]
fn line_ending_auto_detects_crlf_from_input() {
    use cmakefmt::LineEnding;
    let src = "message(hello)\r\nset(X 1)\r\n";
    let config = Config {
        line_ending: LineEnding::Auto,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("\r\n"),
        "auto should preserve CRLF from input"
    );
}

#[test]
fn line_ending_auto_detects_lf_from_input() {
    use cmakefmt::LineEnding;
    let src = "message(hello)\nset(X 1)\n";
    let config = Config {
        line_ending: LineEnding::Auto,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        !formatted.contains("\r\n"),
        "auto should preserve LF from input"
    );
}

#[test]
fn always_wrap_forces_vertical_layout() {
    // Without always_wrap this fits on one line; with it it must be vertical.
    let src = "message(hello world)\n";
    let config = Config {
        always_wrap: vec!["message".to_string()],
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.lines().count() > 1,
        "always_wrap should force multiline output"
    );
}

#[test]
fn require_valid_layout_errors_on_overlong_line() {
    use cmakefmt::Error;
    let src =
        "message(\"this is a very very very very very very long argument that exceeds limit\")\n";
    let config = Config {
        line_width: 40,
        require_valid_layout: true,
        ..Config::default()
    };
    let result = format_source(src, &config);
    assert!(
        matches!(result, Err(Error::LayoutTooWide { .. })),
        "expected LayoutTooWide error, got: {result:?}"
    );
}

#[test]
fn require_valid_layout_passes_when_lines_fit() {
    let src = "message(hello)\n";
    let config = Config {
        line_width: 40,
        require_valid_layout: true,
        ..Config::default()
    };
    assert!(format_source(src, &config).is_ok());
}

#[test]
fn fractional_tab_policy_use_space_preserves_remainder() {
    use cmakefmt::FractionalTabPolicy;
    // "set(" is 4 chars wide; with tab_size=3 the hanging-wrap continuation
    // is 4 spaces = 1 full tab + 1 fractional space.
    // UseSpace keeps the fractional space, so continuation lines start with "\t ".
    let src = "set(AVAR BVAR CVAR DVAR)\n";
    let config = Config {
        use_tabchars: true,
        tab_size: 3,
        line_width: 15, // forces multi-line hanging wrap
        fractional_tab_policy: FractionalTabPolicy::UseSpace,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let has_tab_space = formatted.lines().any(|l| l.starts_with("\t "));
    assert!(
        has_tab_space,
        "use-space should produce tab+space for fractional indentation, got:\n{formatted}"
    );
}

#[test]
fn fractional_tab_policy_round_up_promotes_to_tab() {
    use cmakefmt::FractionalTabPolicy;
    // Same setup: "set(" = 4 chars, tab_size=3 → 1 full tab + 1 fractional.
    // RoundUp promotes the fractional space to an extra tab, so continuation
    // lines start with "\t\t" rather than "\t ".
    let src = "set(AVAR BVAR CVAR DVAR)\n";
    let config = Config {
        use_tabchars: true,
        tab_size: 3,
        line_width: 15,
        fractional_tab_policy: FractionalTabPolicy::RoundUp,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let has_double_tab = formatted.lines().any(|l| l.starts_with("\t\t"));
    assert!(
        has_double_tab,
        "round-up should promote fractional space to extra tab, got:\n{formatted}"
    );
}

// --- Existing tests ---

#[test]
fn bracket_arguments_force_multiline_layout() {
    let src = "set(VAR [==[\nline one\nline two\n]==])\n";

    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @"
    set(
      VAR
      [==[
    line one
    line two
    ]==])
    ");
}
