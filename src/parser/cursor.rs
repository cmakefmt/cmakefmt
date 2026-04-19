// SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[derive(Clone, Copy, Debug)]
pub(super) struct Cursor<'a> {
    source: &'a [u8],
    pos: u32,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
        }
    }

    pub(super) fn pos(&self) -> u32 {
        self.pos
    }

    pub(super) fn peek(&self) -> Option<u8> {
        self.peek_at(0)
    }

    pub(super) fn peek_at(&self, offset: u32) -> Option<u8> {
        self.source
            .get(self.pos as usize + offset as usize)
            .copied()
    }

    pub(super) fn bump(&mut self) {
        if !self.at_eof() {
            self.pos += 1;
        }
    }

    pub(super) fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn at_eof(&self) -> bool {
        self.pos as usize >= self.source.len()
    }

    pub(super) fn consume_bom(&mut self) {
        if self.pos == 0
            && self.peek() == Some(0xEF)
            && self.peek_at(1) == Some(0xBB)
            && self.peek_at(2) == Some(0xBF)
        {
            self.pos = 3;
        }
    }

    pub(super) fn set_pos(&mut self, pos: u32) {
        self.pos = pos.min(self.source.len() as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_bom_advances_only_at_start() {
        let mut cursor = Cursor::new("\u{FEFF}project(foo)");
        cursor.consume_bom();
        assert_eq!(cursor.peek(), Some(b'p'));
    }

    #[test]
    fn peek_and_bump_walk_source() {
        let mut cursor = Cursor::new("ab");
        assert_eq!(cursor.peek(), Some(b'a'));
        cursor.bump();
        assert_eq!(cursor.peek(), Some(b'b'));
        cursor.bump();
        assert!(cursor.at_eof());
    }
}
