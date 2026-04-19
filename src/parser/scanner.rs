// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::cursor::Cursor;
use super::{ScanError, Span};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum ArgumentKind {
    Bracket { level: u32, raw: Span },
    Quoted(Span),
    Unquoted(Span),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum CommentKind {
    Line(Span),
    Bracket(Span),
}

pub(super) fn scan_identifier(c: &mut Cursor<'_>) -> Span {
    let start = c.pos();
    debug_assert!(matches!(
        c.peek(),
        Some(b'_') | Some(b'A'..=b'Z') | Some(b'a'..=b'z')
    ));
    c.bump();
    while matches!(
        c.peek(),
        Some(b'_') | Some(b'0'..=b'9') | Some(b'A'..=b'Z') | Some(b'a'..=b'z')
    ) {
        c.bump();
    }
    Span {
        start,
        end: c.pos(),
    }
}

pub(super) fn scan_comment(c: &mut Cursor<'_>) -> Result<CommentKind, ScanError> {
    let start = c.pos();
    debug_assert_eq!(c.peek(), Some(b'#'));
    c.bump();
    if let Some(level) = bracket_opener_level(c) {
        consume_bracket_open(c, level);
        scan_bracket_close(c, level, "unterminated bracket comment")?;
        return Ok(CommentKind::Bracket(Span {
            start,
            end: c.pos(),
        }));
    }

    while let Some(byte) = c.peek() {
        if byte == b'\n' || (byte == b'\r' && c.peek_at(1) == Some(b'\n')) {
            break;
        }
        c.bump();
    }

    Ok(CommentKind::Line(Span {
        start,
        end: c.pos(),
    }))
}

pub(super) fn scan_argument(c: &mut Cursor<'_>) -> Result<ArgumentKind, ScanError> {
    if bracket_opener_level(c).is_some() {
        let (level, raw) = scan_bracket_literal(c)?;
        return Ok(ArgumentKind::Bracket { level, raw });
    }

    if c.peek() == Some(b'"') {
        let saved = c.pos();
        let span = scan_quoted_literal(c)?;
        if is_argument_boundary(c.peek(), c.peek_at(1)) {
            return Ok(ArgumentKind::Quoted(span));
        }
        c.set_pos(saved);
    }

    Ok(ArgumentKind::Unquoted(scan_unquoted_span(c)?))
}

pub(super) fn scan_template_placeholder(c: &mut Cursor<'_>) -> Result<Span, ScanError> {
    let start = c.pos();
    debug_assert_eq!(c.peek(), Some(b'@'));
    c.bump();

    let mut saw_ident = false;
    while matches!(
        c.peek(),
        Some(b'_') | Some(b'0'..=b'9') | Some(b'A'..=b'Z') | Some(b'a'..=b'z')
    ) {
        saw_ident = true;
        c.bump();
    }

    if !saw_ident || !c.eat(b'@') {
        return Err(ScanError::new("unterminated template placeholder", start));
    }

    Ok(Span {
        start,
        end: c.pos(),
    })
}

pub(super) fn skip_horizontal_space(c: &mut Cursor<'_>) {
    while matches!(c.peek(), Some(b' ' | b'\t')) {
        c.bump();
    }
}

pub(super) fn scan_newline(c: &mut Cursor<'_>) {
    match c.peek() {
        Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
            c.bump();
            c.bump();
        }
        Some(b'\n') => c.bump(),
        _ => debug_assert!(false, "scan_newline called at non-newline byte"),
    }
}

fn scan_quoted_literal(c: &mut Cursor<'_>) -> Result<Span, ScanError> {
    let start = c.pos();
    debug_assert_eq!(c.peek(), Some(b'"'));
    c.bump();

    loop {
        match c.peek() {
            None => return Err(ScanError::new("unterminated quoted argument", start)),
            Some(b'"') => {
                c.bump();
                return Ok(Span {
                    start,
                    end: c.pos(),
                });
            }
            Some(b'\\') => scan_quoted_escape(c, start)?,
            Some(b'$') if is_variable_ref(c) => scan_variable_ref(c)?,
            Some(b'$') if is_genex(c) => scan_genex(c)?,
            _ => c.bump(),
        }
    }
}

fn scan_unquoted_span(c: &mut Cursor<'_>) -> Result<Span, ScanError> {
    let start = c.pos();
    let mut paren_depth = 0u32;
    let mut saw_any = false;

    loop {
        match c.peek() {
            None => {
                if paren_depth > 0 {
                    return Err(ScanError::new(
                        "unterminated parenthesized expression",
                        start,
                    ));
                }
                break;
            }
            Some(b' ' | b'\t') if paren_depth == 0 => break,
            Some(b'\n') if paren_depth == 0 => break,
            Some(b'\r') if paren_depth == 0 && c.peek_at(1) == Some(b'\n') => break,
            Some(b')') if paren_depth == 0 => break,
            Some(b'#') if paren_depth == 0 => break,
            Some(b'\\') => {
                scan_unquoted_escape(c)?;
                saw_any = true;
            }
            Some(b'$') if is_variable_ref(c) => {
                scan_variable_ref(c)?;
                saw_any = true;
            }
            Some(b'$') if is_genex(c) => {
                scan_genex(c)?;
                saw_any = true;
            }
            Some(b'$') if c.peek_at(1) == Some(b'(') => {
                scan_legacy_make_var(c, start)?;
                saw_any = true;
            }
            Some(b'$') => {
                c.bump();
                saw_any = true;
            }
            Some(b'"') => {
                scan_legacy_quoted(c, start)?;
                saw_any = true;
            }
            Some(b'(') => {
                paren_depth += 1;
                c.bump();
                saw_any = true;
            }
            Some(b')') => {
                paren_depth -= 1;
                c.bump();
                saw_any = true;
            }
            Some(b'#') => {
                scan_line_comment_inside_parens(c);
                saw_any = true;
            }
            _ => {
                c.bump();
                saw_any = true;
            }
        }
    }

    if !saw_any {
        return Err(ScanError::new("expected argument", start));
    }

    Ok(Span {
        start,
        end: c.pos(),
    })
}

fn scan_bracket_literal(c: &mut Cursor<'_>) -> Result<(u32, Span), ScanError> {
    let start = c.pos();
    let Some(level) = bracket_opener_level(c) else {
        return Err(ScanError::new("expected bracket argument", start));
    };
    consume_bracket_open(c, level);
    scan_bracket_close(c, level, "unterminated bracket argument")?;
    Ok((
        level,
        Span {
            start,
            end: c.pos(),
        },
    ))
}

fn bracket_opener_level(c: &Cursor<'_>) -> Option<u32> {
    if c.peek() != Some(b'[') {
        return None;
    }

    let mut offset = 1u32;
    while c.peek_at(offset) == Some(b'=') {
        offset += 1;
    }

    (c.peek_at(offset) == Some(b'[')).then_some(offset - 1)
}

fn consume_bracket_open(c: &mut Cursor<'_>, level: u32) {
    c.bump();
    for _ in 0..level {
        debug_assert_eq!(c.peek(), Some(b'='));
        c.bump();
    }
    debug_assert_eq!(c.peek(), Some(b'['));
    c.bump();
}

fn scan_bracket_close(
    c: &mut Cursor<'_>,
    level: u32,
    message: &'static str,
) -> Result<(), ScanError> {
    while let Some(byte) = c.peek() {
        if byte == b']' {
            let mut offset = 1u32;
            while c.peek_at(offset) == Some(b'=') {
                offset += 1;
            }
            if offset - 1 == level && c.peek_at(offset) == Some(b']') {
                for _ in 0..=level + 1 {
                    c.bump();
                }
                return Ok(());
            }
        }
        c.bump();
    }
    Err(ScanError::new(message, c.pos()))
}

fn scan_unquoted_escape(c: &mut Cursor<'_>) -> Result<(), ScanError> {
    let start = c.pos();
    debug_assert_eq!(c.peek(), Some(b'\\'));
    c.bump();
    match c.peek() {
        Some(b't' | b'r' | b'n' | b';') => {
            c.bump();
            Ok(())
        }
        Some(next) if !next.is_ascii_alphanumeric() && next != b';' => {
            c.bump();
            Ok(())
        }
        None => Err(ScanError::new("invalid escape sequence", start)),
        _ => Err(ScanError::new("invalid escape sequence", start)),
    }
}

fn scan_quoted_escape(c: &mut Cursor<'_>, start: u32) -> Result<(), ScanError> {
    debug_assert_eq!(c.peek(), Some(b'\\'));
    c.bump();
    match c.peek() {
        Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
            c.bump();
            c.bump();
            Ok(())
        }
        Some(_) => {
            c.bump();
            Ok(())
        }
        None => Err(ScanError::new("unterminated quoted argument", start)),
    }
}

fn scan_legacy_quoted(c: &mut Cursor<'_>, start: u32) -> Result<(), ScanError> {
    debug_assert_eq!(c.peek(), Some(b'"'));
    c.bump();
    loop {
        match c.peek() {
            Some(b'"') => {
                c.bump();
                return Ok(());
            }
            Some(b'\\') => {
                c.bump();
                match c.peek() {
                    Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
                        return Err(ScanError::new("unterminated legacy quoted segment", start));
                    }
                    Some(b'\n') | None => {
                        return Err(ScanError::new("unterminated legacy quoted segment", start));
                    }
                    Some(_) => c.bump(),
                }
            }
            Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
                return Err(ScanError::new("unterminated legacy quoted segment", start));
            }
            Some(b'\n') | None => {
                return Err(ScanError::new("unterminated legacy quoted segment", start));
            }
            _ => c.bump(),
        }
    }
}

fn scan_legacy_make_var(c: &mut Cursor<'_>, start: u32) -> Result<(), ScanError> {
    debug_assert_eq!(c.peek(), Some(b'$'));
    debug_assert_eq!(c.peek_at(1), Some(b'('));
    c.bump();
    c.bump();
    loop {
        match c.peek() {
            Some(b')') => {
                c.bump();
                return Ok(());
            }
            Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
                return Err(ScanError::new("unterminated legacy make variable", start));
            }
            Some(b'\n') | None => {
                return Err(ScanError::new("unterminated legacy make variable", start));
            }
            _ => c.bump(),
        }
    }
}

fn scan_variable_ref(c: &mut Cursor<'_>) -> Result<(), ScanError> {
    let start = c.pos();
    if c.peek_at(1) == Some(b'{') {
        c.bump();
        c.bump();
        scan_var_name(c, start)?;
        if !c.eat(b'}') {
            return Err(ScanError::new("unterminated variable reference", start));
        }
        return Ok(());
    }

    if c.peek_at(1) == Some(b'E')
        && c.peek_at(2) == Some(b'N')
        && c.peek_at(3) == Some(b'V')
        && c.peek_at(4) == Some(b'{')
    {
        for _ in 0..5 {
            c.bump();
        }
        scan_var_name(c, start)?;
        if !c.eat(b'}') {
            return Err(ScanError::new("unterminated variable reference", start));
        }
        return Ok(());
    }

    if c.peek_at(1) == Some(b'C')
        && c.peek_at(2) == Some(b'A')
        && c.peek_at(3) == Some(b'C')
        && c.peek_at(4) == Some(b'H')
        && c.peek_at(5) == Some(b'E')
        && c.peek_at(6) == Some(b'{')
    {
        for _ in 0..7 {
            c.bump();
        }
        scan_var_name(c, start)?;
        if !c.eat(b'}') {
            return Err(ScanError::new("unterminated variable reference", start));
        }
        return Ok(());
    }

    Err(ScanError::new("unterminated variable reference", start))
}

fn scan_var_name(c: &mut Cursor<'_>, start: u32) -> Result<(), ScanError> {
    loop {
        match c.peek() {
            Some(b'}') => return Ok(()),
            None => return Err(ScanError::new("unterminated variable reference", start)),
            Some(b'$') if c.peek_at(1) == Some(b'{') => scan_variable_ref(c)?,
            _ => c.bump(),
        }
    }
}

fn scan_genex(c: &mut Cursor<'_>) -> Result<(), ScanError> {
    let start = c.pos();
    debug_assert_eq!(c.peek(), Some(b'$'));
    debug_assert_eq!(c.peek_at(1), Some(b'<'));
    c.bump();
    c.bump();
    let mut depth = 1u32;
    while let Some(byte) = c.peek() {
        if byte == b'$' && c.peek_at(1) == Some(b'<') {
            depth += 1;
            c.bump();
            c.bump();
            continue;
        }
        if byte == b'>' {
            depth -= 1;
            c.bump();
            if depth == 0 {
                return Ok(());
            }
            continue;
        }
        c.bump();
    }
    Err(ScanError::new("unterminated generator expression", start))
}

fn scan_line_comment_inside_parens(c: &mut Cursor<'_>) {
    debug_assert_eq!(c.peek(), Some(b'#'));
    c.bump();
    while let Some(byte) = c.peek() {
        if byte == b'\n' || (byte == b'\r' && c.peek_at(1) == Some(b'\n')) {
            break;
        }
        c.bump();
    }
}

fn is_variable_ref(c: &Cursor<'_>) -> bool {
    c.peek() == Some(b'$')
        && (c.peek_at(1) == Some(b'{')
            || (c.peek_at(1) == Some(b'E')
                && c.peek_at(2) == Some(b'N')
                && c.peek_at(3) == Some(b'V')
                && c.peek_at(4) == Some(b'{'))
            || (c.peek_at(1) == Some(b'C')
                && c.peek_at(2) == Some(b'A')
                && c.peek_at(3) == Some(b'C')
                && c.peek_at(4) == Some(b'H')
                && c.peek_at(5) == Some(b'E')
                && c.peek_at(6) == Some(b'{')))
}

fn is_genex(c: &Cursor<'_>) -> bool {
    c.peek() == Some(b'$') && c.peek_at(1) == Some(b'<')
}

fn is_argument_boundary(byte: Option<u8>, next: Option<u8>) -> bool {
    match byte {
        None => true,
        Some(b' ' | b'\t' | b'\n' | b')' | b'#') => true,
        Some(b'\r') => next == Some(b'\n'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_argument_stays_quoted_only_with_boundary() {
        let mut cursor = Cursor::new("\"hello\" )");
        assert!(matches!(
            scan_argument(&mut cursor).unwrap(),
            ArgumentKind::Quoted(_)
        ));
    }

    #[test]
    fn mixed_unquoted_falls_back_from_quote_dispatch() {
        let mut cursor = Cursor::new("\"hello\"world");
        assert!(matches!(
            scan_argument(&mut cursor).unwrap(),
            ArgumentKind::Unquoted(_)
        ));
    }

    #[test]
    fn scan_comment_distinguishes_bracket_comment() {
        let mut cursor = Cursor::new("#[[hello]]");
        assert!(matches!(
            scan_comment(&mut cursor).unwrap(),
            CommentKind::Bracket(_)
        ));
    }

    #[test]
    fn unterminated_genex_reports_error() {
        let mut cursor = Cursor::new("$<TARGET_FILE:foo");
        let err = scan_argument(&mut cursor).unwrap_err();
        assert_eq!(err.message, "unterminated generator expression");
    }
}
