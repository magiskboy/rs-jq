pub mod jql;
pub mod json;

#[cfg(feature = "python")]
mod python;

pub trait Structured {
    type ErrorType;

    fn len(&self) -> Result<usize, Self::ErrorType>;
    fn get(&self, key: &str) -> Result<&Self, Self::ErrorType>;
    fn insert(&mut self, key: &str, element: Self) -> Result<(), Self::ErrorType>;
    fn push(&mut self, element: Self) -> Result<(), Self::ErrorType>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Source<'a> {
    pub data: &'a str,
}

impl<'a> Source<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn char_at(&self, byte_index: usize) -> Option<char> {
        self.data.get(byte_index..)?.chars().next()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// 1-based line and column for a byte offset into `source`.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub fn from_byte(source: &str, byte: usize) -> Self {
        let (line, column) = line_col(source, byte);
        Self { line, column }
    }
}

/// Returns 1-based `(line, column)` for a byte index into `source`.
pub fn line_col(source: &str, byte: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (i, ch) in source.char_indices() {
        if i >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}
