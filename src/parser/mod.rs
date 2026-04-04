//! Parser entry points for CMake source text.
//!
//! The grammar is defined in `parser/cmake.pest`, while
//! [`crate::parser::ast`] contains the AST types returned by
//! [`crate::parser::parse()`].

use pest::Parser;
use pest_derive::Parser;

pub mod ast;

/// Internal pest parser generated from `cmake.pest`.
#[derive(Parser)]
#[grammar = "parser/cmake.pest"]
pub struct CmakeParser;

use crate::error::{Error, Result};
use ast::{Argument, BracketArgument, CommandInvocation, Comment, File, Statement};

/// Parse CMake source text into an AST [`File`].
///
/// The returned AST preserves command structure, blank lines, and comments so
/// the formatter can round-trip files with stable semantics.
pub fn parse(source: &str) -> Result<File> {
    let mut pairs =
        CmakeParser::parse(Rule::file, source).map_err(|e| Error::Parse(Box::new(e)))?;
    let file_pair = pairs
        .next()
        .ok_or_else(|| Error::Formatter("parser did not return a file pair".to_owned()))?;

    build_file(file_pair)
}

fn build_file(pair: pest::iterators::Pair<'_, Rule>) -> Result<File> {
    debug_assert_eq!(pair.as_rule(), Rule::file);

    let items = pair.into_inner();
    let mut statements = Vec::with_capacity(items.size_hint().0);
    let mut pending_blank_lines = 0usize;
    let mut line_has_content = false;

    for item in items {
        collect_file_item(
            item,
            &mut statements,
            &mut pending_blank_lines,
            &mut line_has_content,
        )?;
    }

    flush_blank_lines(&mut statements, &mut pending_blank_lines);
    Ok(File { statements })
}

fn collect_file_item(
    item: pest::iterators::Pair<'_, Rule>,
    statements: &mut Vec<Statement>,
    pending_blank_lines: &mut usize,
    line_has_content: &mut bool,
) -> Result<()> {
    match item.as_rule() {
        Rule::file_item => {
            for inner in item.into_inner() {
                collect_file_item(inner, statements, pending_blank_lines, line_has_content)?;
            }
            Ok(())
        }
        Rule::command_invocation => {
            flush_blank_lines(statements, pending_blank_lines);
            statements.push(Statement::Command(build_command(item)?));
            *line_has_content = true;
            Ok(())
        }
        Rule::template_placeholder => {
            flush_blank_lines(statements, pending_blank_lines);
            statements.push(Statement::TemplatePlaceholder(item.as_str().to_owned()));
            *line_has_content = true;
            Ok(())
        }
        Rule::bracket_comment => {
            let comment = Comment::Bracket(item.as_str().to_owned());
            if let Some(comment) = attach_trailing_comment(statements, comment, *line_has_content) {
                flush_blank_lines(statements, pending_blank_lines);
                statements.push(Statement::Comment(comment));
            }
            *line_has_content = true;
            Ok(())
        }
        Rule::line_comment => {
            let comment = Comment::Line(item.as_str().to_owned());
            if let Some(comment) = attach_trailing_comment(statements, comment, *line_has_content) {
                flush_blank_lines(statements, pending_blank_lines);
                statements.push(Statement::Comment(comment));
            }
            *line_has_content = true;
            Ok(())
        }
        Rule::newline => {
            if *line_has_content {
                *line_has_content = false;
            } else {
                *pending_blank_lines += 1;
            }
            Ok(())
        }
        Rule::space | Rule::EOI => Ok(()),
        other => Err(Error::Formatter(format!(
            "unexpected top-level parser rule: {other:?}"
        ))),
    }
}

fn attach_trailing_comment(
    statements: &mut [Statement],
    comment: Comment,
    line_has_content: bool,
) -> Option<Comment> {
    if !line_has_content {
        return Some(comment);
    }

    match statements.last_mut() {
        Some(Statement::Command(command)) if command.trailing_comment.is_none() => {
            command.trailing_comment = Some(comment);
            None
        }
        _ => Some(comment),
    }
}

fn flush_blank_lines(statements: &mut Vec<Statement>, pending_blank_lines: &mut usize) {
    if *pending_blank_lines == 0 {
        return;
    }

    match statements.last_mut() {
        Some(Statement::BlankLines(count)) => *count += *pending_blank_lines,
        _ => statements.push(Statement::BlankLines(*pending_blank_lines)),
    }

    *pending_blank_lines = 0;
}

fn build_command(pair: pest::iterators::Pair<'_, Rule>) -> Result<CommandInvocation> {
    debug_assert_eq!(pair.as_rule(), Rule::command_invocation);

    let span = pair.as_span();
    let mut name = None;
    let mut arguments = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => {
                name = Some(inner.as_str().to_owned());
            }
            Rule::arguments => {
                arguments = build_arguments(inner)?;
            }
            Rule::space => {}
            other => {
                return Err(Error::Formatter(format!(
                    "unexpected command parser rule: {other:?}"
                )));
            }
        }
    }

    Ok(CommandInvocation {
        name: name.ok_or_else(|| Error::Formatter("command missing identifier".to_owned()))?,
        arguments,
        trailing_comment: None,
        span: (span.start(), span.end()),
    })
}

fn build_arguments(pair: pest::iterators::Pair<'_, Rule>) -> Result<Vec<Argument>> {
    debug_assert_eq!(pair.as_rule(), Rule::arguments);

    let inner = pair.into_inner();
    let mut args = Vec::with_capacity(inner.size_hint().0);

    for p in inner {
        collect_argument_part(p, &mut args)?;
    }

    Ok(args)
}

fn collect_argument_part(
    pair: pest::iterators::Pair<'_, Rule>,
    out: &mut Vec<Argument>,
) -> Result<()> {
    match pair.as_rule() {
        Rule::argument_part => {
            for inner in pair.into_inner() {
                collect_argument_part(inner, out)?;
            }
            Ok(())
        }
        Rule::arguments => {
            for inner in pair.into_inner() {
                collect_argument_part(inner, out)?;
            }
            Ok(())
        }
        Rule::argument => {
            let mut inner = pair.into_inner();
            let argument = inner
                .next()
                .ok_or_else(|| Error::Formatter("argument missing child node".to_owned()))?;
            out.push(build_argument(argument)?);
            Ok(())
        }
        Rule::bracket_comment => {
            out.push(Argument::InlineComment(Comment::Bracket(
                pair.as_str().to_owned(),
            )));
            Ok(())
        }
        Rule::line_ending => {
            collect_line_ending_comments(pair, out);
            Ok(())
        }
        Rule::space => Ok(()),
        other => Err(Error::Formatter(format!(
            "unexpected argument parser rule: {other:?}"
        ))),
    }
}

fn collect_line_ending_comments(pair: pest::iterators::Pair<'_, Rule>, out: &mut Vec<Argument>) {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::line_comment {
            out.push(Argument::InlineComment(Comment::Line(
                inner.as_str().to_owned(),
            )));
        }
    }
}

fn build_argument(pair: pest::iterators::Pair<'_, Rule>) -> Result<Argument> {
    match pair.as_rule() {
        Rule::bracket_argument => {
            let raw = pair.as_str().to_owned();
            Ok(Argument::Bracket(validate_bracket_argument(raw)?))
        }
        Rule::quoted_argument => Ok(Argument::Quoted(pair.as_str().to_owned())),
        Rule::mixed_unquoted_argument | Rule::unquoted_argument => {
            Ok(Argument::Unquoted(pair.as_str().to_owned()))
        }
        other => Err(Error::Formatter(format!(
            "unexpected argument rule: {other:?}"
        ))),
    }
}

/// Validate that a bracket argument's opening and closing "=" counts match.
fn validate_bracket_argument(raw: String) -> Result<BracketArgument> {
    let open_equals = raw
        .strip_prefix('[')
        .ok_or_else(|| Error::Formatter("bracket argument missing '[' prefix".to_owned()))?
        .bytes()
        .take_while(|&b| b == b'=')
        .count();

    let close_equals = raw
        .strip_suffix(']')
        .ok_or_else(|| Error::Formatter("bracket argument missing ']' suffix".to_owned()))?
        .bytes()
        .rev()
        .take_while(|&b| b == b'=')
        .count();

    if open_equals != close_equals {
        return Err(Error::Formatter(format!(
            "invalid bracket argument delimiter: {raw}"
        )));
    }

    Ok(BracketArgument {
        level: open_equals,
        raw,
    })
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(err, Error::Formatter(_)));
    }

    #[test]
    fn line_comment_standalone() {
        let f = parse_ok("# this is a comment\n");
        assert!(matches!(
            &f.statements[0],
            Statement::Comment(Comment::Line(_))
        ));
    }

    #[test]
    fn bracket_comment() {
        let f = parse_ok("#[[ multi\nline ]]\n");
        assert!(matches!(
            &f.statements[0],
            Statement::Comment(Comment::Bracket(_))
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
        assert_eq!(cmd.arguments.len(), 5); // mylib PUBLIC dep1 PRIVATE dep2
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
            Argument::InlineComment(Comment::Bracket(_))
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
        assert!(matches!(cmd.trailing_comment, Some(Comment::Line(_))));
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
