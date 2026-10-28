// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
// SPDX-License-Identifier: MIT OR Apache-2.0

// Each test deliberately supplies ranges, not a collection of line numbers.
#![allow(clippy::single_range_in_vec_init)]

use cmakefmt::{
    format_source_ranges_with_registry, parser, semantic::normalize_semantics, CommandRegistry,
    Config,
};

fn format(source: &str, ranges: &[std::ops::RangeInclusive<usize>]) -> String {
    let registry = CommandRegistry::builtins();
    let output =
        format_source_ranges_with_registry(source, &Config::default(), registry, ranges).unwrap();
    assert_eq!(
        normalize_semantics(parser::parse(source).unwrap(), registry),
        normalize_semantics(parser::parse(&output).unwrap(), registry)
    );
    output
}

#[test]
fn command_scope_preserves_legacy_source_and_block_context() {
    let source = "#  legacy   comment\nset( OLD value )\n\nif( ON )\n    message(  STATUS\n       \"hello\" )\nelse( )\n  set( OTHER value )\nendif( )\n";
    let output = format(source, &[6..=6]);
    insta::assert_snapshot!(output, @r#"
    #  legacy   comment
    set( OLD value )

    if( ON )
      message(STATUS "hello")
    else( )
      set( OTHER value )
    endif( )
    "#);
    assert_eq!(format(&output, &[5..=5]), output);
}

#[test]
fn source_coordinates_survive_earlier_command_shrinking() {
    let source = "set( A\n  one )\nset( B untouched )\nset( C\n  three )\n";
    assert_eq!(
        format(source, &[2..=2, 5..=5, 1..=2]),
        "set(A one)\nset( B untouched )\nset(C three)\n"
    );
}

#[test]
fn quoted_bracket_and_escaped_parentheses_are_not_command_boundaries() {
    let source = "message(  STATUS \"é ) (\"\n  [=[)\n(]=] escaped\\) )\nset( LEGACY keep )\n";
    let output = format(source, &[2..=2]);
    assert_ne!(output, source);
    assert!(output.ends_with("set( LEGACY keep )\n"));
}

#[test]
fn trailing_comment_continuations_are_replaced_once() {
    let source = "set( A value ) # first\n               # second\n# independent   comment\nset( OLD keep )\n";
    let output = format(source, &[2..=2]);
    assert_eq!(
        output,
        "set(A value) # first second\n# independent   comment\nset( OLD keep )\n"
    );
    assert_eq!(format(&output, &[1..=1]), output);
}

#[test]
fn bracket_trailing_comments_keep_their_complete_extent() {
    let source = "set( A value ) #[=[tail\ntext]=]\n# untouched   comment\nset( B value )\n";
    let output = format(source, &[2..=2]);
    assert_eq!(
        output,
        "set(A value) #[=[tail\n             text]=]\n# untouched   comment\nset( B value )\n"
    );
}

#[test]
fn whitespace_comment_and_out_of_bounds_ranges_are_noops() {
    let source = "set( A value )\n\n#  keep   comment\nset( B value )";
    assert_eq!(format(source, &[2..=3, 99..=100]), source);
}

#[test]
fn preserves_bom_crlf_and_missing_final_newline() {
    let source = "\u{feff}set( OLD keep )\r\nif( ON )\r\n\tset( A new )\r\nendif( )";
    let output = format(source, &[3..=3]);
    assert_eq!(
        output,
        "\u{feff}set( OLD keep )\r\nif( ON )\r\n  set(A new)\r\nendif( )"
    );
}

#[test]
fn barriers_preserve_disabled_commands_and_carry_enabled_block_depth() {
    for (off, on) in [
        ("# fmt: off", "# fmt: on"),
        ("# cmakefmt: off", "# cmakefmt: on"),
        ("# cmake-format: off", "# cmake-format: on"),
        ("# ~~~", "# ~~~"),
    ] {
        let source =
            format!("if( ON )\n{off}\n    set( KEEP old )\n{on}\n    set( NEW value )\nendif( )\n");
        let output = format(&source, &[3..=5]);
        assert_eq!(
            output,
            format!("if( ON )\n{off}\n    set( KEEP old )\n{on}\n  set(NEW value)\nendif( )\n")
        );
    }
}

#[test]
fn unselected_legacy_layout_cannot_fail_selected_commands() {
    let config = Config {
        line_width: 20,
        require_valid_layout: true,
        ..Config::default()
    };
    let source = "message( STATUS very_long_legacy_argument_that_cannot_fit )\nset( A value )\n";
    let output =
        format_source_ranges_with_registry(source, &config, CommandRegistry::builtins(), &[2..=2])
            .unwrap();
    assert_eq!(
        output,
        "message( STATUS very_long_legacy_argument_that_cannot_fit )\nset(A value)\n"
    );
}

#[test]
fn invalid_library_ranges_are_rejected() {
    for range in [0..=1, std::ops::RangeInclusive::new(3, 2)] {
        assert!(format_source_ranges_with_registry(
            "set(A value)\n",
            &Config::default(),
            CommandRegistry::builtins(),
            &[range]
        )
        .is_err());
    }
}
