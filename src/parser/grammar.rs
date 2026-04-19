// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::cursor::Cursor;
use super::scanner::{self, ArgumentKind, CommentKind};
use super::{ScanError, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParseTree {
    pub(super) items: Vec<PtItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PtItem {
    Command(PtCommand),
    TemplatePlaceholder(Span),
    LineComment(Span),
    BracketComment(Span),
    Newline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PtCommand {
    pub(super) name: Span,
    pub(super) args: Vec<PtArg>,
    pub(super) full_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PtArg {
    Quoted(Span),
    Bracket { level: u32, raw: Span },
    Unquoted(Span),
    InlineComment(PtComment),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PtComment {
    Line(Span),
    Bracket(Span),
}

pub(super) fn parse_file(source: &str) -> Result<ParseTree, ScanError> {
    let mut cursor = Cursor::new(source);
    let mut items = Vec::new();
    cursor.consume_bom();
    parse_file_body(&mut cursor, &mut items)?;
    Ok(ParseTree { items })
}

fn parse_file_body(c: &mut Cursor<'_>, items: &mut Vec<PtItem>) -> Result<(), ScanError> {
    while !c.at_eof() {
        match c.peek() {
            Some(b'\n') => {
                scanner::scan_newline(c);
                items.push(PtItem::Newline);
            }
            Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
                scanner::scan_newline(c);
                items.push(PtItem::Newline);
            }
            Some(b' ' | b'\t') => scanner::skip_horizontal_space(c),
            Some(b'#') => items.push(match scanner::scan_comment(c)? {
                CommentKind::Line(span) => PtItem::LineComment(span),
                CommentKind::Bracket(span) => PtItem::BracketComment(span),
            }),
            Some(b'@') => items.push(PtItem::TemplatePlaceholder(
                scanner::scan_template_placeholder(c)?,
            )),
            Some(b'_') | Some(b'A'..=b'Z') | Some(b'a'..=b'z') => {
                items.push(PtItem::Command(parse_command(c)?));
            }
            Some(_) => return Err(ScanError::new("unexpected byte", c.pos())),
            None => break,
        }
    }
    Ok(())
}

fn parse_command(c: &mut Cursor<'_>) -> Result<PtCommand, ScanError> {
    let start = c.pos();
    let name = scanner::scan_identifier(c);
    scanner::skip_horizontal_space(c);
    expect(c, b'(', "expected '(' after command name")?;
    let args = parse_arguments(c)?;
    expect(c, b')', "expected ')' to close argument list")?;
    Ok(PtCommand {
        name,
        args,
        full_span: Span {
            start,
            end: c.pos(),
        },
    })
}

fn parse_arguments(c: &mut Cursor<'_>) -> Result<Vec<PtArg>, ScanError> {
    let mut args = Vec::new();

    loop {
        match c.peek() {
            Some(b')') => return Ok(args),
            Some(b' ' | b'\t') => scanner::skip_horizontal_space(c),
            Some(b'\n') => scanner::scan_newline(c),
            Some(b'\r') if c.peek_at(1) == Some(b'\n') => scanner::scan_newline(c),
            Some(b'#') => {
                let comment = match scanner::scan_comment(c)? {
                    CommentKind::Line(span) => PtComment::Line(span),
                    CommentKind::Bracket(span) => PtComment::Bracket(span),
                };
                args.push(PtArg::InlineComment(comment));
            }
            None => return Err(ScanError::new("unterminated argument list", c.pos())),
            Some(_) => args.push(match scanner::scan_argument(c)? {
                ArgumentKind::Quoted(span) => PtArg::Quoted(span),
                ArgumentKind::Bracket { level, raw } => PtArg::Bracket { level, raw },
                ArgumentKind::Unquoted(span) => PtArg::Unquoted(span),
            }),
        }
    }
}

fn expect(c: &mut Cursor<'_>, byte: u8, message: &'static str) -> Result<(), ScanError> {
    if c.eat(byte) {
        Ok(())
    } else {
        Err(ScanError::new(message, c.pos()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tree_keeps_newlines_and_comments() {
        let tree = parse_file("set(FOO bar) # trailing\n# standalone\n").unwrap();
        assert!(matches!(tree.items[0], PtItem::Command(_)));
        assert!(matches!(tree.items[1], PtItem::LineComment(_)));
        assert!(matches!(tree.items[2], PtItem::Newline));
        assert!(matches!(tree.items[3], PtItem::LineComment(_)));
    }
}
