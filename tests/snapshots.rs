use std::collections::HashMap;

use cmfmt::{format_source, CaseStyle, Config, DangleAlign, PerCommandConfig};

// --- Comment tests ---

#[test]
fn standalone_line_comment() {
    let src = "# this is a comment\n";
    let formatted = format_source(src, &Config::default()).unwrap();
    insta::assert_snapshot!(formatted, @"# this is a comment");
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
        b.cc
    )
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
      "Second"
    )
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

// --- Existing tests ---

#[test]
fn wraps_keyword_sections() {
    let src = "target_link_libraries(cmfmt PUBLIC fmt::fmt another::very_long_dependency_name PRIVATE helper::runtime_support)\n";
    let config = Config {
        line_width: 48,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @"
    target_link_libraries(
      cmfmt
      PUBLIC
        fmt::fmt another::very_long_dependency_name
      PRIVATE helper::runtime_support
    )
    ");
}

#[test]
fn discriminated_commands_use_selected_form() {
    let src = "install(TARGETS cmfmt helper RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib/static)\n";
    let config = Config {
        line_width: 52,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @"
    install(
      TARGETS cmfmt helper RUNTIME
      DESTINATION bin LIBRARY
      DESTINATION lib ARCHIVE
      DESTINATION lib/static
    )
    ");
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
        PUBLIC foo bar baz qux quux
    )
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
    let mut per_command = HashMap::new();
    per_command.insert(
        "set".to_string(),
        PerCommandConfig {
            command_case: Some(CaseStyle::Upper),
            ..Default::default()
        },
    );
    let config = Config {
        command_case: CaseStyle::Lower,
        per_command,
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
    ]==]
    )
    ");
}
