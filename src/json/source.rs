use std::io::{self, Read};

use crate::Location;

/// Default refill chunk size for streaming JSON input.
pub const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Sliding byte buffer over a [`Read`] source.
///
/// Bytes before the cursor can be discarded once the lexer/parser no longer
/// needs them. Absolute positions stay stable across discards.
#[derive(Debug)]
pub struct SourceBuffer<R: Read> {
    reader: R,
    buf: Vec<u8>,
    /// Absolute byte offset of `buf[0]`.
    pub(crate) base: usize,
    /// Absolute byte offset of the next byte to consume.
    pos: usize,
    eof: bool,
    /// 1-based line of `pos`.
    line: usize,
    /// 1-based column of `pos`.
    column: usize,
    chunk_size: usize,
}

impl<R: Read> SourceBuffer<R> {
    pub fn new(reader: R) -> Self {
        Self::with_chunk_size(reader, DEFAULT_CHUNK_SIZE)
    }

    pub fn with_chunk_size(reader: R, chunk_size: usize) -> Self {
        Self {
            reader,
            buf: Vec::with_capacity(chunk_size),
            base: 0,
            pos: 0,
            eof: false,
            line: 1,
            column: 1,
            chunk_size: chunk_size.max(1),
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn location(&self) -> Location {
        Location {
            line: self.line,
            column: self.column,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_eof(&self) -> bool {
        self.eof && self.pos >= self.end_abs()
    }

    /// Absolute end offset of bytes currently held in the buffer.
    pub fn buffered_end(&self) -> usize {
        self.end_abs()
    }

    /// Bytes from the cursor through the end of the current buffer (not yet discarded).
    pub fn remaining(&self) -> &[u8] {
        let idx = self.index();
        &self.buf[idx..]
    }

    /// Consume `n` bytes at the cursor (must already be buffered).
    pub fn bump_n(&mut self, n: usize) -> io::Result<()> {
        self.bump_bytes(n)
    }

    fn end_abs(&self) -> usize {
        self.base + self.buf.len()
    }

    fn index(&self) -> usize {
        self.pos - self.base
    }

    /// Ensure at least `n` bytes from the cursor are buffered (or EOF).
    pub fn ensure(&mut self, n: usize) -> io::Result<()> {
        while self.pos + n > self.end_abs() && !self.eof {
            self.fill()?;
        }
        Ok(())
    }

    fn fill(&mut self) -> io::Result<()> {
        let mut tmp = vec![0u8; self.chunk_size];
        let n = self.reader.read(&mut tmp)?;
        if n == 0 {
            self.eof = true;
            return Ok(());
        }
        self.buf.extend_from_slice(&tmp[..n]);
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn peek_byte(&mut self) -> io::Result<Option<u8>> {
        self.ensure(1)?;
        if self.pos >= self.end_abs() {
            return Ok(None);
        }
        Ok(Some(self.buf[self.index()]))
    }

    /// Peek the UTF-8 character at the cursor without consuming it.
    pub fn peek_char(&mut self) -> io::Result<Option<char>> {
        self.ensure(1)?;
        if self.pos >= self.end_abs() {
            return Ok(None);
        }
        let idx = self.index();
        let width = utf8_char_width(self.buf[idx]);
        self.ensure(width)?;
        if self.pos + width > self.end_abs() {
            // Truncated UTF-8 at EOF — surface as a single invalid byte via \u{FFFD}-style handling
            // by returning the byte as a char if possible; callers treat non-JSON starters as errors.
            return Ok(Some(char::from(self.buf[idx])));
        }
        let slice = &self.buf[idx..idx + width];
        Ok(std::str::from_utf8(slice)
            .ok()
            .and_then(|s| s.chars().next())
            .or_else(|| Some(char::from(self.buf[idx]))))
    }

    /// Consume one UTF-8 character at the cursor. Returns `None` at EOF.
    pub fn bump_char(&mut self) -> io::Result<Option<char>> {
        let Some(ch) = self.peek_char()? else {
            return Ok(None);
        };
        let len = ch.len_utf8();
        self.bump_bytes(len)?;
        Ok(Some(ch))
    }

    /// Consume exactly one byte at the cursor (must already be available).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn bump_byte(&mut self) -> io::Result<Option<u8>> {
        self.ensure(1)?;
        if self.pos >= self.end_abs() {
            return Ok(None);
        }
        let b = self.buf[self.index()];
        self.bump_bytes(1)?;
        Ok(Some(b))
    }

    fn bump_bytes(&mut self, n: usize) -> io::Result<()> {
        self.ensure(n)?;
        for _ in 0..n {
            if self.pos >= self.end_abs() {
                break;
            }
            let b = self.buf[self.index()];
            self.pos += 1;
            if b == b'\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        Ok(())
    }

    /// Slice buffered bytes for an absolute `[start, end)` range.
    ///
    /// Panics if the range has been discarded or is not yet buffered.
    pub fn slice_abs(&self, start: usize, end: usize) -> &[u8] {
        assert!(start >= self.base, "slice start was discarded");
        assert!(end <= self.end_abs(), "slice end not buffered");
        assert!(start <= end);
        let s = start - self.base;
        let e = end - self.base;
        &self.buf[s..e]
    }

    /// Discard all buffered bytes with absolute offset `< abs_pos`.
    /// Does not move the cursor; cursor must be `>= abs_pos`.
    pub fn discard_through(&mut self, abs_pos: usize) {
        if abs_pos <= self.base {
            return;
        }
        assert!(
            abs_pos <= self.pos,
            "cannot discard past cursor (abs_pos={abs_pos}, pos={})",
            self.pos
        );
        let drop = abs_pos - self.base;
        self.buf.drain(..drop);
        self.base = abs_pos;
    }

    /// Drop bytes already consumed (everything before the cursor).
    pub fn discard_consumed(&mut self) {
        self.discard_through(self.pos);
    }
}

impl SourceBuffer<&[u8]> {
    pub fn from_str(s: &str) -> SourceBuffer<&[u8]> {
        SourceBuffer::with_chunk_size(s.as_bytes(), s.len().max(1))
    }
}

fn utf8_char_width(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn peek_and_bump_ascii() {
        let mut src = SourceBuffer::from_str("ab\nc");
        assert_eq!(src.peek_byte().unwrap(), Some(b'a'));
        assert_eq!(src.bump_char().unwrap(), Some('a'));
        assert_eq!(src.location(), Location { line: 1, column: 2 });
        assert_eq!(src.bump_char().unwrap(), Some('b'));
        assert_eq!(src.bump_char().unwrap(), Some('\n'));
        assert_eq!(src.location(), Location { line: 2, column: 1 });
        assert_eq!(src.bump_char().unwrap(), Some('c'));
        assert_eq!(src.bump_char().unwrap(), None);
        assert!(src.is_eof());
    }

    #[test]
    fn peek_and_bump_utf8() {
        let mut src = SourceBuffer::from_str("é🙂");
        assert_eq!(src.bump_char().unwrap(), Some('é'));
        assert_eq!(src.position(), 2);
        assert_eq!(src.bump_char().unwrap(), Some('🙂'));
        assert_eq!(src.position(), 6);
        assert_eq!(src.bump_char().unwrap(), None);
    }

    #[test]
    fn fill_from_reader_in_chunks() {
        let data = b"0123456789abcdef";
        let mut src = SourceBuffer::with_chunk_size(Cursor::new(data.as_slice()), 4);
        let mut out = Vec::new();
        while let Some(b) = src.bump_byte().unwrap() {
            out.push(b);
        }
        assert_eq!(out, data);
    }

    #[test]
    fn discard_through_keeps_absolute_positions() {
        let mut src = SourceBuffer::from_str("abcdefghij");
        for _ in 0..6 {
            src.bump_byte().unwrap();
        }
        assert_eq!(src.position(), 6);
        src.discard_through(4);
        assert_eq!(src.base, 4);
        assert_eq!(src.position(), 6);
        assert_eq!(src.peek_byte().unwrap(), Some(b'g'));
        assert_eq!(src.slice_abs(4, 6), b"ef");
    }

    #[test]
    fn discard_consumed() {
        let mut src = SourceBuffer::from_str("hello");
        src.bump_char().unwrap();
        src.bump_char().unwrap();
        src.discard_consumed();
        assert_eq!(src.base, 2);
        assert_eq!(src.slice_abs(2, 5), b"llo");
    }

    #[test]
    fn ensure_grows_for_long_token() {
        let mut big = String::from("\"");
        big.push_str(&"x".repeat(100_000));
        big.push('"');
        let mut src = SourceBuffer::with_chunk_size(Cursor::new(big.as_bytes()), 64);
        src.ensure(big.len()).unwrap();
        assert_eq!(src.end_abs(), big.len());
        assert_eq!(src.slice_abs(0, 1), b"\"");
        assert_eq!(src.slice_abs(big.len() - 1, big.len()), b"\"");
    }
}
