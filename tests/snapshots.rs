// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;
use std::path::PathBuf;

use cmakefmt::spec::registry::CommandRegistry;
use cmakefmt::{
    format_source, format_source_with_registry, CaseStyle, Config, ContinuationAlign, DangleAlign,
    PerCommandConfig,
};
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
    // set() has wrap_after_first_arg=true in the builtin spec, so the
    // variable name stays on the set( line even in vertical mode.
    let src = "set(AVERYLONG_VAR_NAME_HERE B C D E F)\n";
    let config = Config {
        max_rows_cmdline: 1,
        line_width: 35,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    let first_line = formatted.lines().next().unwrap();
    assert_eq!(
        first_line, "set(AVERYLONG_VAR_NAME_HERE",
        "expected variable name on set( line, got:\n{formatted}"
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
        enable_markup: true,
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
        a.cc # keep this
        b.cc)
    ");
}

#[test]
fn inline_bracket_comment_between_arguments() {
    let src = "message(\"First\" #[[inline comment]] \"Second\")\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    message(
      "First" #[[inline comment]]
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
      RUNTIME DESTINATION bin
      LIBRARY DESTINATION lib
      ARCHIVE DESTINATION lib/static)
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
fn install_targets_subkwarg_value_does_not_collide_with_ancestor_kwarg() {
    // `COMPONENT Runtime` inside the RUNTIME artifact-kind subgroup must
    // keep `Runtime` as the COMPONENT value, not re-open a RUNTIME section.
    let src = "install(TARGETS myExe mySharedLib myStaticLib RUNTIME COMPONENT Runtime LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development ARCHIVE COMPONENT Development DESTINATION lib/static FILE_SET HEADERS COMPONENT Development)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS myExe mySharedLib myStaticLib
      RUNTIME COMPONENT Runtime
      LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development
      ARCHIVE COMPONENT Development DESTINATION lib/static
      FILE_SET HEADERS COMPONENT Development)
    ");
}

#[test]
fn install_targets_pair_aware_wrap_at_narrow_width() {
    // At a narrow width the LIBRARY / ARCHIVE sections must wrap with
    // each nested subkwarg (+ its value) on its own line, not split
    // arbitrarily by token count.
    let src = "install(TARGETS myExe mySharedLib myStaticLib RUNTIME COMPONENT Runtime LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development ARCHIVE COMPONENT Development DESTINATION lib/static FILE_SET HEADERS COMPONENT Development)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS myExe mySharedLib myStaticLib
      RUNTIME COMPONENT Runtime
      LIBRARY
        COMPONENT Runtime
        NAMELINK_COMPONENT Development
      ARCHIVE
        COMPONENT Development
        DESTINATION lib/static
      FILE_SET HEADERS COMPONENT Development)
    ");
}

#[test]
fn install_targets_playground_preset_keeps_artifact_subkwargs_grouped() {
    let src = "\
cmake_minimum_required(VERSION 3.25)
project(InstallDemo LANGUAGES CXX)

add_library(widget SHARED src/widget.cpp src/platform.cpp)

install(TARGETS widget EXPORT WidgetTargets
  RUNTIME DESTINATION bin COMPONENT Runtime
  LIBRARY DESTINATION lib COMPONENT Runtime NAMELINK_COMPONENT Development
  ARCHIVE DESTINATION lib COMPONENT Development
  PUBLIC_HEADER DESTINATION include/widget COMPONENT Development
  FILE_SET HEADERS DESTINATION include/widget COMPONENT Development
)
";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    cmake_minimum_required(VERSION 3.25)
    project(InstallDemo LANGUAGES CXX)

    add_library(widget SHARED src/widget.cpp src/platform.cpp)

    install(
      TARGETS widget
      EXPORT WidgetTargets
      RUNTIME DESTINATION bin COMPONENT Runtime
      LIBRARY DESTINATION lib COMPONENT Runtime NAMELINK_COMPONENT Development
      ARCHIVE DESTINATION lib COMPONENT Development
      PUBLIC_HEADER DESTINATION include/widget COMPONENT Development
      FILE_SET HEADERS DESTINATION include/widget COMPONENT Development)
    ");
}

#[test]
fn comments_playground_preset_preserves_barriers_and_reflows_comments() {
    let src = "\
# Project-level note that is intentionally long enough to reflow when markup handling is enabled and the configured line width is tight.
cmake_minimum_required(VERSION 3.20)
project(CommentDemo)

#[=[
This bracket comment should stay attached to the following target.
]=]
add_library(commented src/main.cpp src/detail.cpp) # trailing note that will wrap under the comment column

# cmakefmt: off
set(KEEP_THIS   exactly    as-written)
# cmakefmt: on

########################################
# Generated sources
########################################
target_sources(commented PRIVATE generated/a.cpp generated/b.cpp)
";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @"
    # Project-level note that is intentionally long enough to reflow when markup
    # handling is enabled and the configured line width is tight.
    cmake_minimum_required(VERSION 3.20)
    project(CommentDemo)

    #[=[
    This bracket comment should stay attached to the following target.
    ]=]
    add_library(commented src/main.cpp src/detail.cpp) # trailing note that will
                                                       # wrap under the comment
                                                       # column

    # cmakefmt: off
    set(KEEP_THIS   exactly    as-written)
    # cmakefmt: on

    ########################################
    # Generated sources
    ########################################
    target_sources(commented PRIVATE generated/a.cpp generated/b.cpp)
    ");
}

#[test]
fn wrapping_playground_preset_shows_default_wrap_behavior() {
    let src = "\
cmake_minimum_required(VERSION 3.24)
project(WrapDemo)

target_link_libraries(wrapdemo
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
  PRIVATE project_warnings project_options vendor::long_dependency_name
)

set_property(TARGET wrapdemo PROPERTY
  INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas cxx_variadic_templates)
";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    cmake_minimum_required(VERSION 3.24)
    project(WrapDemo)

    target_link_libraries(
      wrapdemo
      PUBLIC
        Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
      PRIVATE project_warnings project_options vendor::long_dependency_name)

    set_property(
      TARGET wrapdemo
      PROPERTY
        INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas
        cxx_variadic_templates)
    ");
}

#[test]
fn wide_config_playground_preset_keeps_wrapping_example_more_compact() {
    let src = "\
cmake_minimum_required(VERSION 3.24)
project(WrapDemo)

target_link_libraries(wrapdemo
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
  PRIVATE project_warnings project_options vendor::long_dependency_name
)

set_property(TARGET wrapdemo PROPERTY
  INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas cxx_variadic_templates)
";
    let config = Config {
        line_width: 100,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    cmake_minimum_required(VERSION 3.24)
    project(WrapDemo)

    target_link_libraries(
      wrapdemo
      PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
      PRIVATE project_warnings project_options vendor::long_dependency_name)

    set_property(
      TARGET wrapdemo
      PROPERTY INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas cxx_variadic_templates)
    ");
}

#[test]
fn cmake_format_config_playground_preset_dangles_wrapped_parens() {
    let src = "\
cmake_minimum_required(VERSION 3.24)
project(WrapDemo)

target_link_libraries(wrapdemo
  PUBLIC Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
  PRIVATE project_warnings project_options vendor::long_dependency_name
)

set_property(TARGET wrapdemo PROPERTY
  INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas cxx_variadic_templates)
";
    let config = Config {
        continuation_align: ContinuationAlign::UnderFirstValue,
        dangle_parens: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    cmake_minimum_required(VERSION 3.24)
    project(WrapDemo)

    target_link_libraries(
      wrapdemo
      PUBLIC
        Boost::filesystem Boost::system fmt::fmt spdlog::spdlog range-v3::range-v3
      PRIVATE project_warnings project_options vendor::long_dependency_name
    )

    set_property(
      TARGET wrapdemo
      PROPERTY
        INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas
        cxx_variadic_templates
    )
    ");
}

#[test]
fn custom_command_config_playground_preset_groups_project_command_kwargs() {
    let src = "\
CMAKE_MINIMUM_REQUIRED(VERSION 3.20)
PROJECT(MyProject LANGUAGES CXX)

find_package(Boost REQUIRED COMPONENTS filesystem system)

add_library(mylib STATIC
  src/main.cpp
  src/utils.cpp
  src/helper.cpp
)

target_link_libraries(mylib PUBLIC Boost::filesystem Boost::system)
target_include_directories(mylib PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

# Custom function defined by the project
my_add_test(
  NAME mylib_test
  SOURCES tests/test_main.cpp tests/test_utils.cpp
  LIBRARIES mylib
  TIMEOUT 30
  VERBOSE
)

if(BUILD_TESTING)
  enable_testing()
  add_test(NAME integration COMMAND mylib_test --verbose)
endif()
";
    let mut registry = CommandRegistry::load().unwrap();
    registry
        .merge_yaml_overrides(
            "\
commands:
  my_add_test:
    pargs: 0
    flags:
      - VERBOSE
    kwargs:
      NAME:
        nargs: 1
      SOURCES:
        nargs: \"+\"
      LIBRARIES:
        nargs: \"+\"
      TIMEOUT:
        nargs: 1
",
        )
        .unwrap();
    let formatted = format_source_with_registry(src, &Config::default(), &registry).unwrap();

    insta::assert_snapshot!(formatted, @r"
    cmake_minimum_required(VERSION 3.20)
    project(MyProject LANGUAGES CXX)

    find_package(Boost REQUIRED COMPONENTS filesystem system)

    add_library(mylib STATIC src/main.cpp src/utils.cpp src/helper.cpp)

    target_link_libraries(mylib PUBLIC Boost::filesystem Boost::system)
    target_include_directories(mylib PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

    # Custom function defined by the project
    my_add_test(
      NAME mylib_test
      SOURCES tests/test_main.cpp tests/test_utils.cpp
      LIBRARIES mylib
      TIMEOUT 30
      VERBOSE)

    if(BUILD_TESTING)
      enable_testing()
      add_test(NAME integration COMMAND mylib_test --verbose)
    endif()
    ");
}

#[test]
fn install_targets_three_nested_subkwargs_wrap_vertically() {
    // With three subkwargs in a single artifact-kind subgroup, at a width
    // that cannot fit them all inline, each pair lands on its own line.
    let src = "install(TARGETS foo LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development DESTINATION lib CONFIGURATIONS Release)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS foo
      LIBRARY
        COMPONENT Runtime
        NAMELINK_COMPONENT Development
        DESTINATION lib
        CONFIGURATIONS Release)
    ");
}

#[test]
fn install_targets_file_set_with_positional_set_name_and_subkwargs() {
    // FILE_SET has nargs=1 (set name) plus nested subkwargs. The set
    // name must stay attached to FILE_SET and COMPONENT's value must
    // not be reinterpreted.
    let src = "install(TARGETS foo FILE_SET HEADERS DESTINATION include COMPONENT Development CONFIGURATIONS Release)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS foo
      FILE_SET HEADERS
        DESTINATION include
        COMPONENT Development
        CONFIGURATIONS Release)
    ");
}

#[test]
fn install_directory_pattern_subgroup_fits_inline_at_default_width() {
    // When the entire PATTERN subgroup (positional + subkwarg + values)
    // fits within the line-width budget, the formatter keeps it on a
    // single line rather than forcing a vertical split.
    let src = "install(DIRECTORY src/ DESTINATION include PATTERN *.internal EXCLUDE PATTERN *.h PERMISSIONS OWNER_READ OWNER_WRITE)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      DIRECTORY src/
      DESTINATION include
      PATTERN *.internal EXCLUDE
      PATTERN *.h PERMISSIONS OWNER_READ OWNER_WRITE)
    ");
}

#[test]
fn install_directory_pattern_subgroup_pairs_exclude_and_permissions() {
    // PATTERN takes nargs=1 (the glob) and accepts nested EXCLUDE flag
    // plus PERMISSIONS subkwarg. Both must stay grouped under PATTERN.
    let src = "install(DIRECTORY src/ DESTINATION include PATTERN *.internal EXCLUDE PATTERN *.h PERMISSIONS OWNER_READ OWNER_WRITE)\n";
    let config = Config {
        line_width: 45,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      DIRECTORY src/
      DESTINATION include
      PATTERN *.internal EXCLUDE
      PATTERN *.h
        PERMISSIONS OWNER_READ OWNER_WRITE)
    ");
}

#[test]
fn install_directory_pattern_permissions_values_wrap_within_subgroup() {
    // When PERMISSIONS has enough values to overflow line-width, the
    // continuation stays within the PATTERN subgroup — the values do
    // not escape back to the outer DIRECTORY level and PERMISSIONS
    // stays grouped with PATTERN. Under the default
    // `continuation_align = UnderFirstValue`, the continuation
    // aligns under OWNER_EXECUTE's column.
    let src = "install(DIRECTORY src/ DESTINATION include PATTERN *.internal EXCLUDE PATTERN *.h PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ)\n";
    let config = Config {
        line_width: 60,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      DIRECTORY src/
      DESTINATION include
      PATTERN *.internal EXCLUDE
      PATTERN *.h
        PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ
                    GROUP_EXECUTE GROUP_READ)
    ");
}

#[test]
fn install_targets_one_or_more_kwarg_value_does_not_collide_with_kwarg_name() {
    // CONFIGURATIONS has nargs="+" (OneOrMore). Its first value must be
    // force-consumed even when that value spells a kwarg name like
    // `Runtime` — otherwise the config name gets reinterpreted as the
    // RUNTIME artifact-kind subgroup.
    let src = "install(TARGETS foo CONFIGURATIONS Runtime COMPONENT dev)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(
        formatted,
        @"install(TARGETS foo CONFIGURATIONS Runtime COMPONENT dev)"
    );
}

#[test]
fn grouped_writer_does_not_count_comments_toward_subkwarg_nargs() {
    // An inline comment between a nested subkwarg and its value must
    // not be counted as the value — the real value (`Runtime`) must
    // stay grouped with COMPONENT so a following sibling subkwarg
    // (`DESTINATION lib`) opens its own group.
    let src =
        "install(TARGETS foo LIBRARY COMPONENT # component name\n  Runtime DESTINATION lib)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS foo
      LIBRARY
        COMPONENT # component name
                  Runtime
        DESTINATION lib)
    ");
}

#[test]
fn continuation_align_default_hangs_under_first_value() {
    // Default continuation_align mode: wrap lines of a subkwarg
    // group land under the column of the first value after the
    // subkwarg (cmake-format's hanging-indent style).
    let src = "install(DIRECTORY src/ DESTINATION include PATTERN *.internal EXCLUDE PATTERN *.h PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ)\n";
    let config = Config {
        line_width: 60,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      DIRECTORY src/
      DESTINATION include
      PATTERN *.internal EXCLUDE
      PATTERN *.h
        PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ
                    GROUP_EXECUTE GROUP_READ)
    ");
}

#[test]
fn continuation_align_same_indent_wraps_at_subkwarg_indent() {
    // Opt-in continuation_align = SameIndent: wrap lines of a
    // subkwarg group land at the subkwarg's own indent. Consistent
    // with how the rest of the formatter wraps flat-list sections.
    let src = "install(DIRECTORY src/ DESTINATION include PATTERN *.internal EXCLUDE PATTERN *.h PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ GROUP_EXECUTE GROUP_READ)\n";
    let config = Config {
        line_width: 60,
        continuation_align: ContinuationAlign::SameIndent,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      DIRECTORY src/
      DESTINATION include
      PATTERN *.internal EXCLUDE
      PATTERN *.h
        PERMISSIONS OWNER_EXECUTE OWNER_WRITE OWNER_READ
        GROUP_EXECUTE GROUP_READ)
    ");
}

#[test]
fn continuation_align_does_not_affect_inline_fits() {
    // When the group fits on one line, the continuation indent has
    // no effect — both modes produce identical output.
    let src = "install(TARGETS foo LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development)\n";
    for align in [
        ContinuationAlign::SameIndent,
        ContinuationAlign::UnderFirstValue,
    ] {
        let config = Config {
            continuation_align: align,
            ..Config::default()
        };
        let formatted = format_source(src, &config).unwrap();
        assert_eq!(
            formatted,
            "install(TARGETS foo LIBRARY COMPONENT Runtime NAMELINK_COMPONENT Development)\n",
            "unexpected output for {align:?}:\n{formatted}"
        );
    }
}

#[test]
fn autosort_does_not_scramble_structural_kwarg_sections() {
    // Autosort must never flat-sort tokens inside a kwarg section
    // whose spec declares required header positionals, nested
    // subkwargs, or nested flags. Sorting them would detach
    // positionals like FILE_SET's set name from the header or
    // separate subkwargs from their values.
    let src = "install(TARGETS foo FILE_SET HEADERS DESTINATION include COMPONENT Development)\n";
    let config = Config {
        enable_sort: true,
        autosort: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(
        formatted,
        @"install(TARGETS foo FILE_SET HEADERS DESTINATION include COMPONENT Development)"
    );
}

#[test]
fn autosort_does_not_reorder_property_label_sections() {
    // `set_property(... PROPERTY <name> <values…>)` has positional
    // semantics: the first token after PROPERTY is the property name,
    // the rest are its values. Autosort must not flat-sort across
    // that boundary — doing so would silently change the command's
    // meaning by promoting a value into the property-name slot.
    let src = "set_property(TARGET wrapdemo PROPERTY INTERFACE_COMPILE_FEATURES cxx_std_20 cxx_constexpr cxx_lambdas cxx_variadic_templates)\n";
    let config = Config {
        enable_sort: true,
        autosort: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("PROPERTY\n    INTERFACE_COMPILE_FEATURES")
            || formatted.contains("PROPERTY INTERFACE_COMPILE_FEATURES"),
        "INTERFACE_COMPILE_FEATURES must remain the first token after PROPERTY; got:\n{formatted}"
    );
}

#[test]
fn autosort_does_not_reorder_set_target_properties_pairs() {
    // `set_target_properties(... PROPERTIES <k1> <v1> <k2> <v2> …)`
    // has pair semantics. Flat sorting would scramble keys and values
    // across pair boundaries.
    let src = "set_target_properties(mylib PROPERTIES CXX_STANDARD 20 VERSION 1.2.3 SOVERSION 1)\n";
    let config = Config {
        enable_sort: true,
        autosort: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    assert!(
        formatted.contains("CXX_STANDARD 20")
            && formatted.contains("VERSION 1.2.3")
            && formatted.contains("SOVERSION 1"),
        "PROPERTIES key/value pairs must stay paired; got:\n{formatted}"
    );
}

#[test]
fn autosort_still_sorts_flat_positional_sections() {
    // Regression: autosort must keep sorting legitimate flat
    // positional sections (e.g. target_link_libraries PUBLIC list)
    // whose spec has no header positionals, no nested kwargs, and no
    // nested flags.
    let src = "target_link_libraries(mylib PUBLIC zlib boost fmt)\n";
    let config = Config {
        enable_sort: true,
        autosort: true,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(
        formatted,
        @"target_link_libraries(mylib PUBLIC boost fmt zlib)"
    );
}

#[test]
fn install_targets_long_trailing_comment_on_kwarg_breaks_and_reflows() {
    // A trailing comment attached to a header kwarg that would push
    // the line past the configured line_width must not be emitted
    // inline. It breaks to its own line at the nested indent and is
    // reflowed through the shared comment formatter.
    let src = "install(TARGETS foo FILE_SET HEADERS # this is a much longer file set comment with many blahhhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh\n  COMPONENT Development)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS foo
      FILE_SET HEADERS
        # this is a much longer file set comment with many blahhhh blahhh blahhh
        # blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh blahhh
        COMPONENT Development)
    ");
}

#[test]
fn install_targets_prefix_comment_does_not_swallow_required_positional() {
    // A `#` line comment appearing *before* a header kwarg's required
    // positional must not land on the same output line before the
    // positional — CMake would read the positional as part of the
    // comment. FILE_SET's set name `HEADERS` is the canonical case.
    let src = "install(TARGETS foo FILE_SET # file set comment\n  HEADERS COMPONENT Development)\n";
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS foo
      FILE_SET HEADERS # file set comment
        COMPONENT Development)
    ");
}

#[test]
fn install_targets_trailing_comment_on_artifact_kind_stays_on_header_line() {
    // Inline comments that appear immediately after an artifact-kind
    // kwarg (e.g. `RUNTIME # Following options apply to runtime`)
    // stay on the header line when the section wraps, rather than
    // floating to their own line above the grouped subkwargs.
    let src = concat!(
        "install(TARGETS myExe mySharedLib myStaticLib ",
        "RUNTIME # Following options apply to runtime artifacts.\n",
        "COMPONENT Runtime ",
        "LIBRARY # Following options apply to library artifacts.\n",
        "COMPONENT Runtime NAMELINK_COMPONENT Development ",
        "ARCHIVE # Following options apply to archive artifacts.\n",
        "COMPONENT Development DESTINATION lib/static ",
        "FILE_SET HEADERS # Following options apply to file set HEADERS.\n",
        "COMPONENT Development)\n",
    );
    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r"
    install(
      TARGETS myExe mySharedLib myStaticLib
      RUNTIME # Following options apply to runtime artifacts.
        COMPONENT Runtime
      LIBRARY # Following options apply to library artifacts.
        COMPONENT Runtime
        NAMELINK_COMPONENT Development
      ARCHIVE # Following options apply to archive artifacts.
        COMPONENT Development
        DESTINATION lib/static
      FILE_SET HEADERS # Following options apply to file set HEADERS.
        COMPONENT Development)
    ");
}

#[test]
fn non_nested_keyword_section_still_packs_flatly() {
    // target_link_libraries PUBLIC/PRIVATE/INTERFACE have no nested
    // kwargs declared — the pair-aware writer must not kick in here,
    // so existing flat packing behavior is preserved.
    let src =
        "target_link_libraries(mylib PUBLIC dep1 dep2 dep3 dep4 dep5 dep6 dep7 dep8 dep9 dep10)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r"
    target_link_libraries(
      mylib
      PUBLIC
        dep1
        dep2
        dep3
        dep4
        dep5
        dep6
        dep7
        dep8
        dep9
        dep10)
    ");
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

// --- Phase 47g Tier 1 builtins (Pass 1 of CMake spec coverage) ---

#[test]
fn mark_as_advanced_force_flag_separates_when_wrapping() {
    // line_width forces a wrap; FORCE recognised as a flag should land on
    // its own line above the variable list, not be packed inline as a
    // positional. Prior to Phase 47g (pargs: "*") FORCE had no special
    // status and would pack with the variables.
    let src = "mark_as_advanced(FORCE LONG_VAR_ONE LONG_VAR_TWO LONG_VAR_THREE LONG_VAR_FOUR)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    mark_as_advanced(
      FORCE
        LONG_VAR_ONE LONG_VAR_TWO
        LONG_VAR_THREE LONG_VAR_FOUR)
    ");
}

#[test]
fn include_directories_flags_recognized_when_wrapping() {
    // BEFORE and SYSTEM are flags; once wrapped they should be visibly
    // separate from the directory positional list.
    let src = "include_directories(BEFORE SYSTEM /usr/local/include/path1 /opt/include/path2 /opt/include/path3)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    include_directories(
      BEFORE
      SYSTEM
        /usr/local/include/path1
        /opt/include/path2
        /opt/include/path3)
    ");
}

#[test]
fn link_directories_after_flag_separates_when_wrapping() {
    let src =
        "link_directories(AFTER /usr/lib/path1 /usr/lib/path2 /opt/lib/path3 /opt/lib/path4)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    link_directories(
      AFTER
        /usr/lib/path1 /usr/lib/path2
        /opt/lib/path3 /opt/lib/path4)
    ");
}

// --- Phase 47g Tier 2/3 builtins (Pass 1 of CMake spec coverage) ---

#[test]
fn target_compile_features_visibility_kwargs_separate_when_wrapping() {
    // PUBLIC and PRIVATE recognised as visibility kwargs; once wrapped they
    // become separate keyword sections each with their feature list. With
    // the prior pargs: "*" stub, all tokens packed flat.
    let src = "target_compile_features(mylib PUBLIC cxx_std_17 cxx_constexpr cxx_lambdas PRIVATE cxx_inline_namespaces)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    target_compile_features(
      mylib
      PUBLIC cxx_std_17 cxx_constexpr cxx_lambdas
      PRIVATE cxx_inline_namespaces)
    ");
}

#[test]
fn set_tests_properties_kwargs_separate_when_wrapping() {
    let src = "set_tests_properties(mytest other_test third_test DIRECTORY some_dir PROPERTIES TIMEOUT 30 LABELS \"slow\" WILL_FAIL TRUE)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    set_tests_properties(
      mytest other_test third_test
      DIRECTORY some_dir
      PROPERTIES
        TIMEOUT 30 LABELS "slow" WILL_FAIL TRUE)
    "#);
}

#[test]
fn define_property_scope_flags_separate_when_wrapping() {
    let src = "define_property(TARGET PROPERTY MY_LONG_PROPERTY_NAME INHERITED BRIEF_DOCS \"Brief docs about the property\" FULL_DOCS \"Full docs go here\")\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    define_property(
      TARGET
      PROPERTY MY_LONG_PROPERTY_NAME
      INHERITED
      BRIEF_DOCS "Brief docs about the property"
      FULL_DOCS "Full docs go here")
    "#);
}

#[test]
fn ctest_test_kwargs_and_flags_separate_when_wrapping() {
    let src = "ctest_test(BUILD ${CTEST_BINARY_DIRECTORY} PARALLEL_LEVEL 4 STOP_ON_FAILURE INCLUDE_LABEL \"unit\" RETURN_VALUE result_var)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    ctest_test(
      BUILD ${CTEST_BINARY_DIRECTORY}
      PARALLEL_LEVEL 4
      STOP_ON_FAILURE
      INCLUDE_LABEL "unit"
      RETURN_VALUE result_var)
    "#);
}

#[test]
fn add_test_name_form_kwargs_separate_when_wrapping() {
    let src = "add_test(NAME my_long_test_name COMMAND my_executable arg1 arg2 arg3 CONFIGURATIONS Debug Release WORKING_DIRECTORY /tmp/test)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    add_test(
      NAME my_long_test_name
      COMMAND my_executable arg1 arg2 arg3
      CONFIGURATIONS Debug Release
      WORKING_DIRECTORY /tmp/test)
    ");
}

#[test]
fn add_test_legacy_form_packs_all_args_as_positionals() {
    // Legacy form (no NAME discriminator) — fallback DEFAULT form treats
    // everything as positionals, no kwarg recognition.
    let src = "add_test(simple_test_name my_executable arg1 arg2 arg3 arg4 arg5 arg6)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    add_test(
      simple_test_name
      my_executable
      arg1
      arg2
      arg3
      arg4
      arg5
      arg6)
    ");
}

#[test]
fn source_group_tree_form_kwargs_separate_when_wrapping() {
    let src = "source_group(TREE ${CMAKE_CURRENT_SOURCE_DIR}/include PREFIX HeaderFiles FILES alpha.h beta.h gamma.h delta.h)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    source_group(
      TREE ${CMAKE_CURRENT_SOURCE_DIR}/include
      PREFIX HeaderFiles
      FILES alpha.h beta.h gamma.h delta.h)
    ");
}

#[test]
fn source_group_default_form_kwargs_separate_when_wrapping() {
    let src = "source_group(\"Source Files\" FILES main.cpp util.cpp helper.cpp REGULAR_EXPRESSION \".*\\\\.cpp$\")\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    // Verifies the fallback form is correctly applied (source_group's
    // first arg "Source Files" is not the TREE discriminator, so the
    // fallback FILES/REGULAR_EXPRESSION kwargs apply). Earlier the spec
    // used a "DEFAULT" form name that wasn't recognised by form_for(),
    // which silently fell back to the first form (TREE) where
    // REGULAR_EXPRESSION was not a kwarg.
    insta::assert_snapshot!(formatted, @r#"
    source_group(
      "Source Files"
      FILES main.cpp util.cpp helper.cpp
      REGULAR_EXPRESSION ".*\\.cpp$")
    "#);
}

#[test]
fn cmake_path_get_form_recognizes_modifier_flags() {
    // GET form: pargs=2 (path-var, out-var) plus kind-keyword flag.
    // Verifies that EXTENSION + LAST_ONLY are both recognised as flags.
    let src = "cmake_path(GET FOO_PATH_VAR EXTENSION LAST_ONLY FOO_EXT_OUT_VARIABLE)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    cmake_path(
      GET FOO_PATH_VAR
      EXTENSION
      LAST_ONLY FOO_EXT_OUT_VARIABLE)
    ");
}

#[test]
fn cmake_path_absolute_path_form_kwargs_separate_when_wrapping() {
    let src = "cmake_path(ABSOLUTE_PATH RELATIVE_INPUT BASE_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR} NORMALIZE OUTPUT_VARIABLE ABS_RESULT)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    cmake_path(
      ABSOLUTE_PATH RELATIVE_INPUT
      BASE_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
      NORMALIZE
      OUTPUT_VARIABLE ABS_RESULT)
    ");
}

// --- Phase 54 module command coverage (Pass 1 of CMake module spec) ---

#[test]
fn fetchcontent_getproperties_kwargs_separate_when_wrapping() {
    let src = "FetchContent_GetProperties(googletest SOURCE_DIR gtest_src BINARY_DIR gtest_build POPULATED gtest_done)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    fetchcontent_getproperties(
      googletest
      SOURCE_DIR gtest_src
      BINARY_DIR gtest_build
      POPULATED gtest_done)
    ");
}

#[test]
fn pkg_get_variable_define_variables_kwarg_recognized() {
    let src = "pkg_get_variable(LIB_PREFIX libssl prefix DEFINE_VARIABLES PREFIX=/usr/local INCLUDEDIR=/usr/local/include)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    pkg_get_variable(
      LIB_PREFIX libssl prefix
      DEFINE_VARIABLES
        PREFIX=/usr/local
        INCLUDEDIR=/usr/local/include)
    ");
}

#[test]
fn externalproject_add_kwargs_separate_when_wrapping() {
    let src = "ExternalProject_Add(myExtProj GIT_REPOSITORY https://github.com/example/foo.git GIT_TAG v1.2.3 GIT_SHALLOW TRUE CMAKE_ARGS -DBUILD_SHARED_LIBS=ON BUILD_COMMAND ${CMAKE_COMMAND} --build . LOG_BUILD ON)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    externalproject_add(
      myExtProj
      GIT_REPOSITORY https://github.com/example/foo.git
      GIT_TAG v1.2.3
      GIT_SHALLOW TRUE
      CMAKE_ARGS -DBUILD_SHARED_LIBS=ON
      BUILD_COMMAND ${CMAKE_COMMAND} --build .
      LOG_BUILD ON)
    ");
}

#[test]
fn fetchcontent_declare_recognizes_flags_and_kwargs() {
    let src = "FetchContent_Declare(googletest GIT_REPOSITORY https://github.com/google/googletest.git GIT_TAG v1.14.0 SYSTEM EXCLUDE_FROM_ALL FIND_PACKAGE_ARGS NAMES GTest)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    fetchcontent_declare(
      googletest
      GIT_REPOSITORY https://github.com/google/googletest.git
      GIT_TAG v1.14.0
      SYSTEM
      EXCLUDE_FROM_ALL
      FIND_PACKAGE_ARGS NAMES GTest)
    ");
}

#[test]
fn check_ipo_supported_kwargs_separate_when_wrapping() {
    // Wrap pressure forces RESULT/OUTPUT/LANGUAGES kwargs onto their own
    // lines. Without the spec they would all pack into one positional list
    // — the kwarg recognition is what gives them structural separation.
    let src = "check_ipo_supported(RESULT IPO_OK OUTPUT IPO_ERROR LANGUAGES C CXX Fortran)\n";
    let config = Config {
        line_width: 40,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    check_ipo_supported(
      RESULT IPO_OK
      OUTPUT IPO_ERROR
      LANGUAGES C CXX Fortran)
    ");
}

#[test]
fn find_dependency_kwargs_separate_when_wrapping() {
    let src = "find_dependency(Boost 1.70 REQUIRED COMPONENTS filesystem system iostreams program_options OPTIONAL_COMPONENTS regex)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    find_dependency(
      Boost 1.70
      REQUIRED
      COMPONENTS filesystem system iostreams program_options
      OPTIONAL_COMPONENTS regex)
    ");
}

#[test]
fn gtest_discover_tests_kwargs_separate_when_wrapping() {
    let src = "gtest_discover_tests(my_test EXTRA_ARGS --verbose --color=yes WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR} TEST_PREFIX MyTest. PROPERTIES TIMEOUT 30 LABELS unit DISCOVERY_TIMEOUT 60)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    gtest_discover_tests(
      my_test
      EXTRA_ARGS --verbose --color=yes
      WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
      TEST_PREFIX MyTest.
      PROPERTIES TIMEOUT 30 LABELS unit
      DISCOVERY_TIMEOUT 60)
    ");
}

#[test]
fn externalproject_get_property_packs_property_list_under_wrap() {
    // Variadic positionals should pack horizontally up to the line
    // budget rather than going one-per-line. With seven properties and
    // a narrow line_width, the formatter still wraps them as a flat
    // packed list rather than as a kwarg-style vertical block.
    let src = "ExternalProject_Get_Property(myExtProj SOURCE_DIR BINARY_DIR INSTALL_DIR STAMP_DIR DOWNLOAD_DIR LOG_DIR TMP_DIR)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    externalproject_get_property(
      myExtProj
      SOURCE_DIR
      BINARY_DIR
      INSTALL_DIR
      STAMP_DIR
      DOWNLOAD_DIR
      LOG_DIR
      TMP_DIR)
    ");
}

#[test]
fn check_include_file_packs_when_inline() {
    // Representative of the ~25 trivial-pargs Check* commands. With
    // three positionals and a comfortable line_width, the call fits
    // on one line. The spec entry is `pargs: "2+"` which does not
    // change the layout for trivial cases — registry-level tests
    // assert the spec exists; this snapshot anchors the layout.
    let src = "check_include_file(sys/types.h HAVE_SYS_TYPES_H \"-D_GNU_SOURCE\")\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(
        formatted,
        @r#"check_include_file(sys/types.h HAVE_SYS_TYPES_H "-D_GNU_SOURCE")"#
    );
}

#[test]
fn configure_package_config_file_kwargs_and_flags_separate_when_wrapping() {
    let src = "configure_package_config_file(MyLibConfig.cmake.in MyLibConfig.cmake INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/MyLib PATH_VARS CMAKE_INSTALL_INCLUDEDIR CMAKE_INSTALL_LIBDIR NO_SET_AND_CHECK_MACRO NO_CHECK_REQUIRED_COMPONENTS_MACRO)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    configure_package_config_file(
      MyLibConfig.cmake.in MyLibConfig.cmake
      INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/MyLib
      PATH_VARS CMAKE_INSTALL_INCLUDEDIR CMAKE_INSTALL_LIBDIR
      NO_SET_AND_CHECK_MACRO
      NO_CHECK_REQUIRED_COMPONENTS_MACRO)
    ");
}

#[test]
fn write_basic_package_version_file_arch_independent_flag_separates() {
    // The canonical "release a CMake package" pattern: ARCH_INDEPENDENT
    // should land on its own line as a flag, distinct from the VERSION
    // and COMPATIBILITY kwargs.
    let src = "write_basic_package_version_file(MyLibConfigVersion.cmake VERSION 2.4.1 COMPATIBILITY SameMajorVersion ARCH_INDEPENDENT)\n";
    let config = Config {
        line_width: 60,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    write_basic_package_version_file(
      MyLibConfigVersion.cmake
      VERSION 2.4.1
      COMPATIBILITY SameMajorVersion
      ARCH_INDEPENDENT)
    ");
}

#[test]
fn pkg_check_modules_recognizes_quiet_required_imported_target_flags() {
    let src =
        "pkg_check_modules(MYDEPS REQUIRED IMPORTED_TARGET libssl libcrypto libpng libjpeg)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    pkg_check_modules(
      MYDEPS
      REQUIRED
      IMPORTED_TARGET libssl libcrypto libpng libjpeg)
    ");
}

#[test]
fn find_package_handle_standard_args_rich_kwargs_and_flags() {
    // The kwarg+flag surface most every CMake project uses through a
    // FindFoo.cmake module. REQUIRED_VARS takes a list, VERSION_VAR
    // takes one value, and HANDLE_VERSION_RANGE / HANDLE_COMPONENTS /
    // CONFIG_MODE are all separate flags that should each land on
    // their own line.
    let src = "find_package_handle_standard_args(MyLib REQUIRED_VARS MyLib_INCLUDE_DIR MyLib_LIBRARY VERSION_VAR MyLib_VERSION HANDLE_VERSION_RANGE HANDLE_COMPONENTS CONFIG_MODE)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    find_package_handle_standard_args(
      MyLib
      REQUIRED_VARS MyLib_INCLUDE_DIR MyLib_LIBRARY
      VERSION_VAR MyLib_VERSION
      HANDLE_VERSION_RANGE
      HANDLE_COMPONENTS
      CONFIG_MODE)
    ");
}

#[test]
fn gtest_add_tests_sister_to_gtest_discover_tests() {
    // gtest_add_tests has pargs: 0 (everything is kwargs/flags), where
    // gtest_discover_tests has pargs: 1 (the target). Both should
    // recognise their kwargs and flags identically.
    let src = "gtest_add_tests(TARGET my_test_target SOURCES test1.cc test2.cc test3.cc EXTRA_ARGS --color=yes WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR} TEST_PREFIX MyLib. SKIP_DEPENDENCY)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    gtest_add_tests(
      TARGET my_test_target
      SOURCES test1.cc test2.cc test3.cc
      EXTRA_ARGS --color=yes
      WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
      TEST_PREFIX MyLib.
      SKIP_DEPENDENCY)
    ");
}

#[test]
fn cmake_parse_arguments_high_pargs_count_packs_when_wrapping() {
    // pargs: "4+" — exercises a shape that no other tested command
    // uses. The four required positionals (prefix, options, one_value,
    // multi_value) plus ${ARGN} sentinel should pack into the
    // available width.
    let src = "cmake_parse_arguments(ARG \"VERBOSE;OPTIONAL\" \"DESTINATION;COMPONENT\" \"SOURCES;DEPENDS\" ${ARGN})\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    cmake_parse_arguments(ARG "VERBOSE;OPTIONAL" "DESTINATION;COMPONENT"
                          "SOURCES;DEPENDS" ${ARGN})
    "#);
}

#[test]
fn externalproject_add_step_kwargs_separate_when_wrapping() {
    // Sister of the already-tested ExternalProject_Add but with
    // pargs: 2 (project name + step name) and a smaller kwarg surface.
    let src = "ExternalProject_Add_Step(myExtProj custom_step COMMAND ${CMAKE_COMMAND} -E echo \"Hello\" DEPENDEES build DEPENDERS install BYPRODUCTS hello.txt WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR} ALWAYS TRUE)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    externalproject_add_step(
      myExtProj custom_step
      COMMAND ${CMAKE_COMMAND} -E echo "Hello"
      DEPENDEES build
      DEPENDERS install
      BYPRODUCTS hello.txt
      WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
      ALWAYS TRUE)
    "#);
}

#[test]
fn cpack_add_component_recognizes_flags_and_kwargs() {
    // cpack_add_component carries four flags (HIDDEN, REQUIRED, DISABLED,
    // DOWNLOADED) alongside seven kwargs. Without the spec, the flags
    // would pack into the positional list; with it, they each get their
    // own line under wrap pressure.
    let src = "cpack_add_component(my_component DISPLAY_NAME \"My Component\" DESCRIPTION \"An example component\" GROUP runtime DEPENDS core utils INSTALL_TYPES Full Developer DOWNLOADED ARCHIVE_FILE my_component.zip)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    cpack_add_component(
      my_component
      DISPLAY_NAME "My Component"
      DESCRIPTION "An example component"
      GROUP runtime
      DEPENDS core utils
      INSTALL_TYPES Full Developer
      DOWNLOADED
      ARCHIVE_FILE my_component.zip)
    "#);
}

#[test]
fn cpack_ifw_configure_component_kwargs_separate_when_wrapping() {
    // CPackIFW's configure_component has a large surface (5 flags, 18
    // kwargs). This test exercises a representative slice and confirms
    // the spec recognises the COMMON / ESSENTIAL / VIRTUAL flags
    // alongside the NAME / VERSION / DEPENDENCIES kwargs.
    let src = "cpack_ifw_configure_component(my_component COMMON ESSENTIAL NAME my.component VERSION 1.2.3 DEPENDENCIES core utils SCRIPT install.qs)\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @r"
    cpack_ifw_configure_component(
      my_component
      COMMON
      ESSENTIAL
      NAME my.component
      VERSION 1.2.3
      DEPENDENCIES core utils
      SCRIPT install.qs)
    ");
}

#[test]
fn fixup_bundle_recognizes_ignore_item_kwarg() {
    // fixup_bundle takes three positionals (app, libs, dirs) plus an
    // optional IGNORE_ITEM kwarg. The kwarg should break to its own
    // line under wrap pressure even when the positionals fit packed.
    let src = "fixup_bundle(${MY_APP} \"plugin1.dylib;plugin2.dylib\" \"/usr/local/lib;/opt/lib\" IGNORE_ITEM vcredist_x86.exe vcredist_x64.exe)\n";
    let config = Config {
        line_width: 50,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r#"
    fixup_bundle(
      ${MY_APP} "plugin1.dylib;plugin2.dylib"
      "/usr/local/lib;/opt/lib"
      IGNORE_ITEM vcredist_x86.exe vcredist_x64.exe)
    "#);
}

#[test]
fn android_add_test_data_kwargs_separate_when_wrapping() {
    // android_add_test_data has seven kwargs, including the variadic
    // FILES / LIBS / NO_LINK_REGEX. Wrap pressure should give each
    // section its own header line.
    let src = "android_add_test_data(my_test FILES data1.txt data2.txt LIBS foo.so bar.so DEVICE_OBJECT_STORE /sdcard/data DEVICE_TEST_DIR /sdcard/test NO_LINK_REGEX .*\\.so)\n";
    let config = Config {
        line_width: 60,
        ..Config::default()
    };
    let formatted = format_source(src, &config).unwrap();
    insta::assert_snapshot!(formatted, @r"
    android_add_test_data(
      my_test
      FILES data1.txt data2.txt
      LIBS foo.so bar.so
      DEVICE_OBJECT_STORE /sdcard/data
      DEVICE_TEST_DIR /sdcard/test
      NO_LINK_REGEX .*\.so)
    ");
}

// --- Existing tests ---

#[test]
fn bracket_arguments_force_multiline_layout() {
    let src = "set(VAR [==[\nline one\nline two\n]==])\n";

    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @"
    set(VAR [==[
    line one
    line two
    ]==])
    ");
}
