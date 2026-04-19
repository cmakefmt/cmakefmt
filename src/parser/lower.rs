// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::ast::{Argument, BracketArgument, CommandInvocation, Comment, File, Statement};
use super::grammar::{ParseTree, PtArg, PtCommand, PtComment, PtItem};

pub(super) fn lower(source: &str, tree: ParseTree) -> File {
    let mut statements = Vec::with_capacity(tree.items.len());
    let mut pending_blank_lines = 0usize;
    let mut line_has_content = false;
    let mut trailing_comment_col: Option<u32> = None;
    let line_starts = build_line_starts(source);

    for item in tree.items {
        match item {
            PtItem::Newline => {
                if line_has_content {
                    line_has_content = false;
                } else {
                    pending_blank_lines += 1;
                    trailing_comment_col = None;
                }
            }
            PtItem::Command(command) => {
                flush_blank_lines(&mut statements, &mut pending_blank_lines);
                statements.push(Statement::Command(lower_command(source, command)));
                line_has_content = true;
                trailing_comment_col = None;
            }
            PtItem::TemplatePlaceholder(span) => {
                flush_blank_lines(&mut statements, &mut pending_blank_lines);
                statements.push(Statement::TemplatePlaceholder(
                    source[span.range()].to_owned(),
                ));
                line_has_content = true;
                trailing_comment_col = None;
            }
            PtItem::BracketComment(span) => {
                trailing_comment_col = None;
                let comment = Comment::Bracket(source[span.range()].to_owned());
                if let Some(comment) =
                    attach_trailing_comment(&mut statements, comment, line_has_content)
                {
                    flush_blank_lines(&mut statements, &mut pending_blank_lines);
                    statements.push(Statement::Comment(comment));
                }
                line_has_content = true;
            }
            PtItem::LineComment(span) => {
                let col = column_of(span.start, source, &line_starts);
                let body = source[span.range()].to_owned();
                let comment = Comment::Line(body.clone());

                if line_has_content {
                    if let Some(comment) =
                        attach_trailing_comment(&mut statements, comment, line_has_content)
                    {
                        flush_blank_lines(&mut statements, &mut pending_blank_lines);
                        statements.push(Statement::Comment(comment));
                        trailing_comment_col = None;
                    } else {
                        trailing_comment_col = Some(col);
                    }
                } else if pending_blank_lines == 0
                    && trailing_comment_col == Some(col)
                    && merge_trailing_comment_continuation(&mut statements, &body)
                {
                } else {
                    trailing_comment_col = None;
                    flush_blank_lines(&mut statements, &mut pending_blank_lines);
                    statements.push(Statement::Comment(Comment::Line(body)));
                }

                line_has_content = true;
            }
        }
    }

    flush_blank_lines(&mut statements, &mut pending_blank_lines);
    File { statements }
}

fn build_line_starts(source: &str) -> Vec<u32> {
    let mut starts = vec![0];
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push((idx + 1) as u32);
        }
    }
    starts
}

fn column_of(offset: u32, source: &str, line_starts: &[u32]) -> u32 {
    let idx = line_starts
        .partition_point(|&start| start <= offset)
        .saturating_sub(1);
    let line_start = line_starts[idx] as usize;
    source[line_start..offset as usize].chars().count() as u32 + 1
}

fn lower_command(source: &str, command: PtCommand) -> CommandInvocation {
    CommandInvocation {
        name: source[command.name.range()].to_owned(),
        arguments: command
            .args
            .into_iter()
            .map(|arg| lower_arg(source, arg))
            .collect(),
        trailing_comment: None,
        span: (
            command.full_span.start as usize,
            command.full_span.end as usize,
        ),
    }
}

fn lower_arg(source: &str, arg: PtArg) -> Argument {
    match arg {
        PtArg::Quoted(span) => Argument::Quoted(source[span.range()].to_owned()),
        PtArg::Unquoted(span) => Argument::Unquoted(source[span.range()].to_owned()),
        PtArg::Bracket { level, raw } => Argument::Bracket(BracketArgument {
            level: level as usize,
            raw: source[raw.range()].to_owned(),
        }),
        PtArg::InlineComment(PtComment::Line(span)) => {
            Argument::InlineComment(Comment::Line(source[span.range()].to_owned()))
        }
        PtArg::InlineComment(PtComment::Bracket(span)) => {
            Argument::InlineComment(Comment::Bracket(source[span.range()].to_owned()))
        }
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

fn merge_trailing_comment_continuation(statements: &mut [Statement], continuation: &str) -> bool {
    let Some(Statement::Command(command)) = statements.last_mut() else {
        return false;
    };
    let Some(Comment::Line(ref mut text)) = command.trailing_comment else {
        return false;
    };
    let body = continuation.trim_start_matches('#').trim_start();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::grammar::{PtCommand, PtItem};
    use crate::parser::Span;

    #[test]
    fn lowering_merges_aligned_trailing_comment_continuations() {
        let source = "set(FOO bar) # first\n             # second\n";
        let tree = ParseTree {
            items: vec![
                PtItem::Command(PtCommand {
                    name: Span { start: 0, end: 3 },
                    args: vec![
                        PtArg::Unquoted(Span { start: 4, end: 7 }),
                        PtArg::Unquoted(Span { start: 8, end: 11 }),
                    ],
                    full_span: Span { start: 0, end: 12 },
                }),
                PtItem::LineComment(Span { start: 13, end: 20 }),
                PtItem::Newline,
                PtItem::LineComment(Span { start: 34, end: 42 }),
                PtItem::Newline,
            ],
        };

        let file = lower(source, tree);
        let Statement::Command(command) = &file.statements[0] else {
            panic!()
        };
        assert_eq!(
            command.trailing_comment,
            Some(Comment::Line("# first second".to_owned()))
        );
    }
}
