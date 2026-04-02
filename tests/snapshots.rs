use cmfmt::{format_source, Config};

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
