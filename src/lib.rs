pub mod jql;
pub mod json;

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

    pub fn slice(&self, start: usize, end: usize) -> Option<&'a str> {
        self.data.get(start..end)
    }

    pub fn slice_at(&self, start: usize) -> Option<&'a str> {
        self.data.get(start..)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
