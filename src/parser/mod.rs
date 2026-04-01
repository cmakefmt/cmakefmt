use pest::Parser;
use pest_derive::Parser;

pub mod ast;

#[derive(Parser)]
#[grammar = "parser/cmake.pest"]
pub struct CmakeParser;

use crate::error::{Error, Result};
use ast::{Argument, BracketArgument, CommandInvocation, Comment, File, Statement};

/// Parse CMake source text into an AST.
pub fn parse(source: &str) -> Result<File> {
    let pairs = CmakeParser::parse(Rule::file, source)
        .map_err(|e| Error::Parse(Box::new(e)))?;

    let file_pair = pairs.into_iter().next().expect("file rule always produces one pair");
    Ok(build_file(file_pair))
}

fn build_file(pair: pest::iterators::Pair<Rule>) -> File {
    debug_assert_eq!(pair.as_rule(), Rule::file);

    let mut statements = Vec::new();

    for element in pair.into_inner() {
        match element.as_rule() {
            Rule::file_element => {
                collect_file_element(element, &mut statements);
            }
            Rule::EOI => {}
            _ => {}
        }
    }

    File { statements }
}

/// Recursively extract statements from a file_element pair.
/// line_comment may be nested inside line_ending, so we descend.
fn collect_file_element(pair: pest::iterators::Pair<Rule>, out: &mut Vec<Statement>) {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::command_invocation => {
                out.push(Statement::Command(build_command(inner)));
            }
            Rule::bracket_comment => {
                out.push(Statement::Comment(Comment::Bracket(inner.as_str().to_owned())));
            }
            Rule::line_comment => {
                out.push(Statement::Comment(Comment::Line(inner.as_str().to_owned())));
            }
            Rule::newline => {
                // A bare newline (with no comment) at the file-element level
                // contributes to blank line counting. We accumulate these.
                // For now, a single newline is just a line separator — not blank.
                // Multiple newlines in a row become BlankLines nodes.
                // (Handled when we see consecutive newline file_elements.)
            }
            Rule::line_ending | Rule::space => {
                // Descend to find any line_comment inside.
                collect_file_element(inner, out);
            }
            _ => {}
        }
    }
}

fn build_command(pair: pest::iterators::Pair<Rule>) -> CommandInvocation {
    debug_assert_eq!(pair.as_rule(), Rule::command_invocation);

    let span = pair.as_span();
    let mut inner = pair.into_inner();

    // Skip leading space if any (the grammar allows space* before identifier).
    let name_pair = inner
        .find(|p| p.as_rule() == Rule::identifier)
        .expect("command_invocation always has an identifier");
    let name = name_pair.as_str().to_owned();

    let args_pair = inner
        .find(|p| p.as_rule() == Rule::arguments)
        .expect("command_invocation always has arguments");
    let arguments = build_arguments(args_pair);

    CommandInvocation {
        name,
        arguments,
        span: (span.start(), span.end()),
    }
}

fn build_arguments(pair: pest::iterators::Pair<Rule>) -> Vec<Argument> {
    debug_assert_eq!(pair.as_rule(), Rule::arguments);

    let mut args = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::argument => {
                let inner = p.into_inner().next().expect("argument has one child");
                args.push(build_argument(inner));
            }
            // Comments that appear between arguments are attached as inline comments.
            Rule::bracket_comment => {
                args.push(Argument::InlineComment(Comment::Bracket(
                    p.as_str().to_owned(),
                )));
            }
            Rule::line_comment => {
                args.push(Argument::InlineComment(Comment::Line(
                    p.as_str().to_owned(),
                )));
            }
            _ => {}
        }
    }

    args
}

fn build_argument(pair: pest::iterators::Pair<Rule>) -> Argument {
    match pair.as_rule() {
        Rule::bracket_argument => {
            let raw = pair.as_str().to_owned();
            // Validate that open/close bracket "=" counts match.
            let arg = validate_bracket_argument(raw);
            Argument::Bracket(arg)
        }
        Rule::quoted_argument => {
            Argument::Quoted(pair.as_str().to_owned())
        }
        Rule::unquoted_argument => {
            Argument::Unquoted(pair.as_str().to_owned())
        }
        r => panic!("unexpected argument rule: {r:?}"),
    }
}

/// Validate that a bracket argument's opening and closing "=" counts match.
/// Panics if they don't (the grammar guarantees they are balanced structurally,
/// but this double-checks correctness during development).
fn validate_bracket_argument(raw: String) -> BracketArgument {
    // raw looks like: [=*[ ... ]=*]
    let open_equals = raw
        .strip_prefix('[')
        .expect("bracket argument starts with [")
        .bytes()
        .take_while(|&b| b == b'=')
        .count();

    let close_equals = raw
        .strip_suffix(']')
        .expect("bracket argument ends with ]")
        .bytes()
        .rev()
        .take_while(|&b| b == b'=')
        .count();

    debug_assert_eq!(
        open_equals, close_equals,
        "bracket argument open/close = count mismatch in: {raw}"
    );

    BracketArgument {
        level: open_equals,
        raw,
    }
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
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert_eq!(cmd.name, "cmake_minimum_required");
        assert_eq!(cmd.arguments.len(), 2);
    }

    #[test]
    fn command_no_args() {
        let f = parse_ok("some_command()\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert!(cmd.arguments.is_empty());
    }

    #[test]
    fn quoted_argument() {
        let f = parse_ok("message(\"hello world\")\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert!(matches!(&cmd.arguments[0], Argument::Quoted(_)));
    }

    #[test]
    fn bracket_argument_zero_equals() {
        let f = parse_ok("set(VAR [[hello]])\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        let Argument::Bracket(b) = &cmd.arguments[1] else { panic!() };
        assert_eq!(b.level, 0);
    }

    #[test]
    fn bracket_argument_one_equals() {
        let f = parse_ok("set(VAR [=[hello]=])\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        let Argument::Bracket(b) = &cmd.arguments[1] else { panic!() };
        assert_eq!(b.level, 1);
    }

    #[test]
    fn bracket_argument_two_equals() {
        let f = parse_ok("set(VAR [==[contains ]= inside]==])\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        let Argument::Bracket(b) = &cmd.arguments[1] else { panic!() };
        assert_eq!(b.level, 2);
    }

    #[test]
    fn line_comment_standalone() {
        let f = parse_ok("# this is a comment\n");
        assert!(matches!(&f.statements[0], Statement::Comment(Comment::Line(_))));
    }

    #[test]
    fn bracket_comment() {
        let f = parse_ok("#[[ multi\nline ]]\n");
        assert!(matches!(&f.statements[0], Statement::Comment(Comment::Bracket(_))));
    }

    #[test]
    fn variable_reference_in_unquoted() {
        let f = parse_ok("message(${MY_VAR})\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert!(matches!(&cmd.arguments[0], Argument::Unquoted(_)));
    }

    #[test]
    fn env_variable_reference() {
        let f = parse_ok("message($ENV{PATH})\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert!(matches!(&cmd.arguments[0], Argument::Unquoted(_)));
    }

    #[test]
    fn generator_expression() {
        let f = parse_ok("target_link_libraries(foo $<TARGET_FILE:bar>)\n");
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert_eq!(cmd.arguments.len(), 2);
    }

    #[test]
    fn multiline_argument_list() {
        let src = "target_link_libraries(mylib\n    PUBLIC dep1\n    PRIVATE dep2\n)\n";
        let f = parse_ok(src);
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert_eq!(cmd.name, "target_link_libraries");
        assert_eq!(cmd.arguments.len(), 5); // mylib PUBLIC dep1 PRIVATE dep2
    }

    #[test]
    fn escape_sequences_in_quoted() {
        let f = parse_ok("message(\"tab\\there\\nnewline\")\n");
        assert!(!f.statements.is_empty());
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
        let Statement::Command(cmd) = &f.statements[0] else { panic!() };
        assert_eq!(cmd.arguments.len(), 1);
    }
}
