use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonParserError {
    LexicalError(String),
    ParserError(String),
    SerializeError(String),
    DeserializeError(String),
}

impl Display for JsonParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LexicalError(msg) => write!(f, "lexical error: {}", msg),
            Self::ParserError(msg) => write!(f, "parse error: {}", msg),
            Self::SerializeError(msg) => write!(f, "serialize error: {}", msg),
            Self::DeserializeError(msg) => write!(f, "deserialize error: {}", msg),
        }
    }
}
