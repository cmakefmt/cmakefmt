use cmfmt::{format_source, Config};

#[test]
fn wraps_keyword_sections() {
    let src = "target_link_libraries(cmfmt PUBLIC fmt::fmt another::very_long_dependency_name PRIVATE helper::runtime_support)\n";
    let config = Config {
        line_width: 48,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    target_link_libraries(
      cmfmt
      PUBLIC
        fmt::fmt another::very_long_dependency_name
      PRIVATE helper::runtime_support
    )
    "#);
}

#[test]
fn discriminated_commands_use_selected_form() {
    let src = "install(TARGETS cmfmt helper RUNTIME DESTINATION bin LIBRARY DESTINATION lib ARCHIVE DESTINATION lib/static)\n";
    let config = Config {
        line_width: 52,
        ..Config::default()
    };

    let formatted = format_source(src, &config).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    install(
      TARGETS cmfmt helper RUNTIME
      DESTINATION bin LIBRARY
      DESTINATION lib ARCHIVE
      DESTINATION lib/static
    )
    "#);
}

#[test]
fn bracket_arguments_force_multiline_layout() {
    let src = "set(VAR [==[\nline one\nline two\n]==])\n";

    let formatted = format_source(src, &Config::default()).unwrap();

    insta::assert_snapshot!(formatted, @r#"
    set(
      VAR
      [==[
    line one
    line two
    ]==]
    )
    "#);
}
