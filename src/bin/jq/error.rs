use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum AppError {
    InvalidFormat,
    InvalidInput,
    ParseError(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid format"),
            Self::InvalidInput => write!(f, "invalid input"),
            Self::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}
