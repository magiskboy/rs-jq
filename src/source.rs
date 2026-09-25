#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
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
