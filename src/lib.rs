pub mod jql;
pub mod json;
mod source;

pub trait Structured {
    type ErrorType;

    fn len(&self) -> Result<usize, Self::ErrorType>;
    fn get(&self, key: &str) -> Result<&Self, Self::ErrorType>;
    fn insert(&mut self, key: &str, element: Self) -> Result<(), Self::ErrorType>;
    fn push(&mut self, element: Self) -> Result<(), Self::ErrorType>;
}
