// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use cmakefmt::parser::ast::{Argument, BracketArgument, CommandInvocation, Comment, File, Statement};
use pest::error::{ErrorVariant, LineColLocation};
use pest::Parser;

mod generated {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "compare/cmake_legacy.pest"]
    pub(super) struct CmakeParser;
}

use generated::{CmakeParser, Rule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyDiagnostic {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

pub(crate) fn parse_reference(source: &str) -> std::result::Result<File, LegacyDiagnostic> {
    parse_internal(source)
}

fn parse_internal(source: &str) -> std::result::Result<File, LegacyDiagnostic> {
    let mut pairs = CmakeParser::parse(Rule::file, source).map_err(from_pest)?;
    let file_pair = pairs.next().ok_or(LegacyDiagnostic { line: 1, column: 1 })?;
    build_file(file_pair)
}

fn from_pest<R: pest::RuleType>(error: pest::error::Error<R>) -> LegacyDiagnostic {
    let (line, column) = match error.line_col {
        LineColLocation::Pos((line, column)) => (line, column),
        LineColLocation::Span((line, column), _) => (line, column),
    };
    let _message = match &error.variant {
        ErrorVariant::ParsingError { positives, .. } if !positives.is_empty() => format!(
            "expected {}",
            positives
                .iter()
                .map(|rule| format!("{rule:?}").replace('_', " "))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ErrorVariant::CustomError { message } => message.clone(),
        _ => error.to_string(),
    };
    LegacyDiagnostic { line, column }
}

fn build_file(pair: pest::iterators::Pair<'_, Rule>) -> std::result::Result<File, LegacyDiagnostic> {
    let items = pair.into_inner();
    let mut statements = Vec::with_capacity(items.size_hint().0);
    let mut pending_blank_lines = 0usize;
    let mut line_has_content = false;
    let mut trailing_comment_col: Option<usize> = None;
    for item in items {
        collect_file_item(
            item,
            &mut statements,
            &mut pending_blank_lines,
            &mut line_has_content,
            &mut trailing_comment_col,
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
    trailing_comment_col: &mut Option<usize>,
) -> std::result::Result<(), LegacyDiagnostic> {
    match item.as_rule() {
        Rule::file_item => {
            for inner in item.into_inner() {
                collect_file_item(
                    inner,
                    statements,
                    pending_blank_lines,
                    line_has_content,
                    trailing_comment_col,
                )?;
            }
            Ok(())
        }
        Rule::command_invocation => {
            *trailing_comment_col = None;
            flush_blank_lines(statements, pending_blank_lines);
            statements.push(Statement::Command(build_command(item)?));
            *line_has_content = true;
            Ok(())
        }
        Rule::template_placeholder => {
            *trailing_comment_col = None;
            flush_blank_lines(statements, pending_blank_lines);
            statements.push(Statement::TemplatePlaceholder(item.as_str().to_owned()));
            *line_has_content = true;
            Ok(())
        }
        Rule::bracket_comment => {
            *trailing_comment_col = None;
            let comment = Comment::Bracket(item.as_str().to_owned());
            if let Some(comment) = attach_trailing_comment(statements, comment, *line_has_content) {
                flush_blank_lines(statements, pending_blank_lines);
                statements.push(Statement::Comment(comment));
            }
            *line_has_content = true;
            Ok(())
        }
        Rule::line_comment => {
            let col = item.as_span().start_pos().line_col().1;
            let comment = Comment::Line(item.as_str().to_owned());
            if *line_has_content {
                if let Some(comment) =
                    attach_trailing_comment(statements, comment, *line_has_content)
                {
                    flush_blank_lines(statements, pending_blank_lines);
                    statements.push(Statement::Comment(comment));
                    *trailing_comment_col = None;
                } else {
                    *trailing_comment_col = Some(col);
                }
            } else if *pending_blank_lines == 0
                && *trailing_comment_col == Some(col)
                && merge_trailing_comment_continuation(statements, &comment)
            {
            } else {
                *trailing_comment_col = None;
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
                *trailing_comment_col = None;
                *pending_blank_lines += 1;
            }
            Ok(())
        }
        Rule::space | Rule::EOI => Ok(()),
        _ => Err(LegacyDiagnostic { line: 1, column: 1 }),
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

fn merge_trailing_comment_continuation(
    statements: &mut [Statement],
    continuation: &Comment,
) -> bool {
    let Some(Statement::Command(command)) = statements.last_mut() else {
        return false;
    };
    let Some(Comment::Line(ref mut text)) = command.trailing_comment else {
        return false;
    };
    let Comment::Line(cont_text) = continuation else {
        return false;
    };
    let body = cont_text.trim_start_matches('#').trim_start();
    if !body.is_empty() {
        text.push(' ');
        text.push_str(body);
    }
    true
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

fn build_command(
    pair: pest::iterators::Pair<'_, Rule>,
) -> std::result::Result<CommandInvocation, LegacyDiagnostic> {
    let span = pair.as_span();
    let mut name = None;
    let mut arguments = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::identifier => name = Some(inner.as_str().to_owned()),
            Rule::arguments => arguments = build_arguments(inner)?,
            Rule::space => {}
            _ => return Err(LegacyDiagnostic { line: 1, column: 1 }),
        }
    }
    Ok(CommandInvocation {
        name: name.unwrap_or_default(),
        arguments,
        trailing_comment: None,
        span: (span.start(), span.end()),
    })
}

fn build_arguments(
    pair: pest::iterators::Pair<'_, Rule>,
) -> std::result::Result<Vec<Argument>, LegacyDiagnostic> {
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
) -> std::result::Result<(), LegacyDiagnostic> {
    match pair.as_rule() {
        Rule::argument_part | Rule::arguments => {
            for inner in pair.into_inner() {
                collect_argument_part(inner, out)?;
            }
            Ok(())
        }
        Rule::argument => {
            if let Some(argument) = pair.into_inner().next() {
                out.push(build_argument(argument)?);
                Ok(())
            } else {
                Err(LegacyDiagnostic { line: 1, column: 1 })
            }
        }
        Rule::bracket_comment => {
            out.push(Argument::InlineComment(Comment::Bracket(pair.as_str().to_owned())));
            Ok(())
        }
        Rule::line_ending => {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::line_comment {
                    out.push(Argument::InlineComment(Comment::Line(inner.as_str().to_owned())));
                }
            }
            Ok(())
        }
        Rule::space => Ok(()),
        _ => Err(LegacyDiagnostic { line: 1, column: 1 }),
    }
}

fn build_argument(
    pair: pest::iterators::Pair<'_, Rule>,
) -> std::result::Result<Argument, LegacyDiagnostic> {
    match pair.as_rule() {
        Rule::bracket_argument => Ok(Argument::Bracket(validate_bracket_argument(
            pair.as_str().to_owned(),
        )?)),
        Rule::quoted_argument => Ok(Argument::Quoted(pair.as_str().to_owned())),
        Rule::mixed_unquoted_argument | Rule::unquoted_argument => {
            Ok(Argument::Unquoted(pair.as_str().to_owned()))
        }
        _ => Err(LegacyDiagnostic { line: 1, column: 1 }),
    }
}

fn validate_bracket_argument(raw: String) -> std::result::Result<BracketArgument, LegacyDiagnostic> {
    let open_equals = raw
        .strip_prefix('[')
        .ok_or(LegacyDiagnostic { line: 1, column: 1 })?
        .bytes()
        .take_while(|&b| b == b'=')
        .count();
    let close_equals = raw
        .strip_suffix(']')
        .ok_or(LegacyDiagnostic { line: 1, column: 1 })?
        .bytes()
        .rev()
        .take_while(|&b| b == b'=')
        .count();
    if open_equals != close_equals {
        return Err(LegacyDiagnostic { line: 1, column: 1 });
    }
    Ok(BracketArgument {
        level: open_equals,
        raw,
    })
}
