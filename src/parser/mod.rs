// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parser entry points for CMake source text.
//!
//! The parser is a hand-written recursive-descent implementation over a
//! streaming scanner. [`crate::parser::ast`] contains the public AST returned by
//! [`crate::parser::parse()`].

use std::ops::Range;

pub mod ast;

mod cursor;
mod grammar;
mod lower;
mod scanner;

use ast::File;

use crate::error::{Error, ParseDiagnostic, ParseError, Result};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) struct Span {
    pub(super) start: u32,
    pub(super) end: u32,
}

impl Span {
    pub(super) fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) struct ScanError {
    pub(super) message: &'static str,
    pub(super) byte_offset: u32,
}

impl ScanError {
    pub(super) fn new(message: &'static str, byte_offset: u32) -> Self {
        Self {
            message,
            byte_offset,
        }
    }
}

/// Parse CMake source text into an AST [`File`].
///
/// The returned AST preserves command structure, blank lines, and comments so
/// the formatter can round-trip files with stable semantics.
pub fn parse(source: &str) -> Result<File> {
    parse_v2(source)
}

pub(crate) fn parse_v2(source: &str) -> Result<File> {
    if source.len() > u32::MAX as usize {
        return Err(Error::Parse(ParseError {
            display_name: "<source>".to_owned(),
            source_text: source.to_owned().into_boxed_str(),
            start_line: 1,
            diagnostic: ParseDiagnostic {
                message: "source exceeds maximum supported size".into(),
                line: 1,
                column: 1,
            },
        }));
    }

    let tree = grammar::parse_file(source).map_err(|e| Error::Parse(to_public_error(e, source)))?;
    Ok(lower::lower(source, tree))
}

fn to_public_error(error: ScanError, source: &str) -> ParseError {
    let (line, column) = line_col_at(source, error.byte_offset);
    ParseError {
        display_name: "<source>".to_owned(),
        source_text: source.to_owned().into_boxed_str(),
        start_line: 1,
        diagnostic: ParseDiagnostic {
            message: error.message.into(),
            line,
            column,
        },
    }
}

pub(super) fn line_col_at(source: &str, offset: u32) -> (usize, usize) {
    let offset = offset as usize;
    let clamped = offset.min(source.len());
    let prefix = &source[..clamped];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let column = source[line_start..clamped].chars().count() + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::ast::{Argument, Statement};
    use super::*;

    fn parse_ok(src: &str) -> File {
        parse(src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
    }

    #[test]
    fn empty_file() {
        let f = parse_ok("");
        assert!(f.statements.is_empty());
    }

    #[test]
    fn simple_command() {
        let f = parse_ok("cmake_minimum_required(VERSION 3.20)\n");
        assert_eq!(f.statements.len(), 1);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.name, "cmake_minimum_required");
        assert_eq!(cmd.arguments.len(), 2);
        assert!(cmd.trailing_comment.is_none());
    }

    #[test]
    fn command_no_args() {
        let f = parse_ok("some_command()\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(cmd.arguments.is_empty());
    }

    #[test]
    fn quoted_argument() {
        let f = parse_ok("message(\"hello world\")\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(matches!(&cmd.arguments[0], Argument::Quoted(_)));
    }

    #[test]
    fn bracket_argument_zero_equals() {
        let f = parse_ok("set(VAR [[hello]])\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let Argument::Bracket(b) = &cmd.arguments[1] else {
            panic!()
        };
        assert_eq!(b.level, 0);
    }

    #[test]
    fn bracket_argument_one_equals() {
        let f = parse_ok("set(VAR [=[hello]=])\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let Argument::Bracket(b) = &cmd.arguments[1] else {
            panic!()
        };
        assert_eq!(b.level, 1);
    }

    #[test]
    fn bracket_argument_two_equals() {
        let f = parse_ok("set(VAR [==[contains ]= inside]==])\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let Argument::Bracket(b) = &cmd.arguments[1] else {
            panic!()
        };
        assert_eq!(b.level, 2);
    }

    #[test]
    fn invalid_bracket_argument_returns_error() {
        let err = parse("set(VAR [=[hello]==])\n").unwrap_err();
        assert!(matches!(err, Error::Parse(_)));
    }

    #[test]
    fn invalid_syntax_returns_parse_error_with_crate_owned_diagnostic() {
        let err = parse("message(\n").unwrap_err();
        let Error::Parse(parse_err) = err else {
            panic!("expected parse error");
        };

        assert_eq!(parse_err.display_name, "<source>");
        assert_eq!(parse_err.source_text.as_ref(), "message(\n");
        assert_eq!(parse_err.start_line, 1);
        assert!(!parse_err.diagnostic.message.is_empty());
        assert_eq!(parse_err.diagnostic.line, 2);
        assert_eq!(parse_err.diagnostic.column, 1);
    }

    #[test]
    fn line_comment_standalone() {
        let f = parse_ok("# this is a comment\n");
        assert!(matches!(
            &f.statements[0],
            Statement::Comment(ast::Comment::Line(_))
        ));
    }

    #[test]
    fn bracket_comment() {
        let f = parse_ok("#[[ multi\nline ]]\n");
        assert!(matches!(
            &f.statements[0],
            Statement::Comment(ast::Comment::Bracket(_))
        ));
    }

    #[test]
    fn variable_reference_in_unquoted() {
        let f = parse_ok("message(${MY_VAR})\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(matches!(&cmd.arguments[0], Argument::Unquoted(_)));
    }

    #[test]
    fn env_variable_reference() {
        let f = parse_ok("message($ENV{PATH})\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(matches!(&cmd.arguments[0], Argument::Unquoted(_)));
    }

    #[test]
    fn generator_expression() {
        let f = parse_ok("target_link_libraries(foo $<TARGET_FILE:bar>)\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.arguments.len(), 2);
    }

    #[test]
    fn multiline_argument_list() {
        let src = "target_link_libraries(mylib\n    PUBLIC dep1\n    PRIVATE dep2\n)\n";
        let f = parse_ok(src);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.name, "target_link_libraries");
        assert_eq!(cmd.arguments.len(), 5);
    }

    #[test]
    fn inline_bracket_comment_in_arguments() {
        let src = "message(\"First\" #[[inline comment]] \"Second\")\n";
        let f = parse_ok(src);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.arguments.len(), 3);
        assert!(matches!(
            &cmd.arguments[1],
            Argument::InlineComment(ast::Comment::Bracket(_))
        ));
    }

    #[test]
    fn line_comment_between_arguments() {
        let src = "target_sources(foo\n  PRIVATE a.cc # keep grouping\n  b.cc\n)\n";
        let f = parse_ok(src);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(cmd.arguments.iter().any(Argument::is_comment));
    }

    #[test]
    fn trailing_comment_after_command() {
        let src = "message(STATUS \"hello\") # trailing\n";
        let f = parse_ok(src);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert!(matches!(cmd.trailing_comment, Some(ast::Comment::Line(_))));
    }

    #[test]
    fn aligned_continuation_merges_into_trailing_comment() {
        let src = "set(FOO bar) # first line\n             # second line\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 1);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.trailing_comment,
            Some(ast::Comment::Line("# first line second line".to_owned()))
        );
    }

    #[test]
    fn multiple_aligned_continuations_merge() {
        let src = "set(FOO bar) # line one\n             # line two\n             # line three\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 1);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.trailing_comment,
            Some(ast::Comment::Line(
                "# line one line two line three".to_owned()
            ))
        );
    }

    #[test]
    fn non_aligned_comment_stays_standalone() {
        let src = "set(FOO bar) # trailing\n# standalone\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 2);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.trailing_comment,
            Some(ast::Comment::Line("# trailing".to_owned()))
        );
        assert!(matches!(f.statements[1], Statement::Comment(_)));
    }

    #[test]
    fn blank_line_prevents_continuation_merge() {
        let src = "set(FOO bar) # trailing\n\n             # not a continuation\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 3);
    }

    #[test]
    fn empty_continuation_line_merges_without_adding_text() {
        let src = "set(FOO bar) # first\n             #\n             # third\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 1);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.trailing_comment,
            Some(ast::Comment::Line("# first third".to_owned()))
        );
    }

    #[test]
    fn off_by_one_column_prevents_merge() {
        let src = "set(FOO bar) # trailing\n              # off by one\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 2);
        assert!(matches!(f.statements[1], Statement::Comment(_)));
    }

    #[test]
    fn file_without_final_newline() {
        let f = parse_ok("project(MyProject)");
        assert_eq!(f.statements.len(), 1);
    }

    #[test]
    fn blank_lines_are_preserved() {
        let f = parse_ok("message(foo)\n\nproject(bar)\n");
        assert_eq!(f.statements.len(), 3);
        assert!(matches!(f.statements[1], Statement::BlankLines(1)));
    }

    #[test]
    fn leading_blank_lines_are_preserved() {
        let f = parse_ok("\nmessage(foo)\n");
        assert!(matches!(f.statements[0], Statement::BlankLines(1)));
    }

    #[test]
    fn escape_sequences_in_quoted() {
        let f = parse_ok("message(\"tab\\there\\nnewline\")\n");
        assert!(!f.statements.is_empty());
    }

    #[test]
    fn escaped_quotes_in_quoted_argument_parse() {
        let f = parse_ok("message(FATAL_ERROR \"foo \\\"Debug\\\"\")\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let args: Vec<&str> = cmd.arguments.iter().map(Argument::as_str).collect();
        assert_eq!(args, vec!["FATAL_ERROR", "\"foo \\\"Debug\\\"\""]);
    }

    #[test]
    fn multiple_commands() {
        let src = "cmake_minimum_required(VERSION 3.20)\nproject(MyProject)\n";
        let f = parse_ok(src);
        assert_eq!(f.statements.len(), 2);
    }

    #[test]
    fn nested_variable_reference() {
        let f = parse_ok("message(${${OUTER}})\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.arguments.len(), 1);
    }

    #[test]
    fn underscore_command_name_is_valid() {
        let f = parse_ok("_my_command(ARG)\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.name, "_my_command");
    }

    #[test]
    fn nested_parentheses_in_arguments_are_preserved_as_unquoted_tokens() {
        let f = parse_ok("if(FALSE AND (FALSE OR TRUE))\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let args: Vec<&str> = cmd.arguments.iter().map(Argument::as_str).collect();
        assert_eq!(args, vec!["FALSE", "AND", "(FALSE OR TRUE)"]);
    }

    #[test]
    fn multiline_nested_parentheses_in_arguments_are_preserved_as_unquoted_tokens() {
        let f = parse_ok(concat!(
            "IF(NOT (have_C__fsanitize_memory__fsanitize_memory_track_origins__U_FORTIFY_SOURCE\n",
            "          AND have_CXX__fsanitize_memory__fsanitize_memory_track_origins__U_FORTIFY_SOURCE))\n",
        ));
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        let args: Vec<&str> = cmd.arguments.iter().map(Argument::as_str).collect();
        assert_eq!(
            args,
            vec![
                "NOT",
                "(have_C__fsanitize_memory__fsanitize_memory_track_origins__U_FORTIFY_SOURCE\n          AND have_CXX__fsanitize_memory__fsanitize_memory_track_origins__U_FORTIFY_SOURCE)"
            ]
        );
    }

    #[test]
    fn source_file_with_utf8_bom_parses() {
        let f = parse_ok("\u{FEFF}project(MyProject)\n");
        assert_eq!(f.statements.len(), 1);
    }

    #[test]
    fn top_level_template_placeholder_parses() {
        let f = parse_ok("@PACKAGE_INIT@\n");
        assert_eq!(
            f.statements,
            vec![Statement::TemplatePlaceholder("@PACKAGE_INIT@".to_owned())]
        );
    }

    #[test]
    fn legacy_unquoted_argument_with_embedded_quotes_parses() {
        let f = parse_ok("set(x -Da=\"b c\")\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.arguments[1].as_str(), "-Da=\"b c\"");
    }

    #[test]
    fn legacy_unquoted_argument_with_make_style_reference_parses() {
        let f = parse_ok("set(x -Da=$(v))\n");
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(cmd.arguments[1].as_str(), "-Da=$(v)");
    }

    #[test]
    fn legacy_unquoted_argument_with_embedded_parens_parses() {
        let f = parse_ok(r##"set(VERSION_REGEX "#define CLI11_VERSION[ 	]+"(.+)"")"##);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.arguments[1].as_str(),
            "\"#define CLI11_VERSION[ \t]+\"(.+)\"\""
        );
    }

    #[test]
    fn legacy_unquoted_argument_starting_with_quoted_segment_parses() {
        let f = parse_ok(r##"list(APPEND force-libcxx "CMAKE_CXX_COMPILER_ID STREQUAL "Clang"")"##);
        let Statement::Command(cmd) = &f.statements[0] else {
            panic!()
        };
        assert_eq!(
            cmd.arguments[2].as_str(),
            "\"CMAKE_CXX_COMPILER_ID STREQUAL \"Clang\"\""
        );
    }
}
