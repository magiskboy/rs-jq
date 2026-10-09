use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use crate::json::token::JsonTokenKind;
use crate::{Location, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedSyntax {
    Value,
    Token(JsonTokenKind),
}

impl ExpectedSyntax {
    fn name(&self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Token(kind) => kind.syntax_name(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonType {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

impl JsonType {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedJsonType {
    Boolean,
    Number,
    Array,
    Object,
    ArrayOrObject,
}

impl ExpectedJsonType {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Array => "array",
            Self::Object => "object",
            Self::ArrayOrObject => "array or object",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonErrorKind {
    InvalidCharacter,
    InvalidLiteral {
        expected: &'static str,
    },
    InvalidNumber,
    UnterminatedString,
    InvalidEscape,
    InvalidUnicodeEscape,
    InvalidSurrogatePair,
    UnescapedControl,
    EmptyInput,
    UnexpectedToken {
        expected: ExpectedSyntax,
        found: JsonTokenKind,
    },
    UnexpectedEof {
        expected: ExpectedSyntax,
    },
    TrailingInput,
    TypeMismatch {
        expected: ExpectedJsonType,
        found: JsonType,
    },
    KeyNotFound {
        key: String,
    },
    IndexOutOfBounds {
        index: usize,
        len: usize,
    },
    InvalidIndex {
        key: String,
    },
    InvalidCast,
    Io {
        message: String,
    },
}

impl JsonErrorKind {
    fn phase(&self) -> &'static str {
        match self {
            Self::InvalidCharacter
            | Self::InvalidLiteral { .. }
            | Self::InvalidNumber
            | Self::UnterminatedString
            | Self::InvalidEscape
            | Self::InvalidUnicodeEscape
            | Self::InvalidSurrogatePair
            | Self::UnescapedControl => "lexical",
            Self::EmptyInput
            | Self::UnexpectedToken { .. }
            | Self::UnexpectedEof { .. }
            | Self::TrailingInput => "parse",
            Self::TypeMismatch { .. }
            | Self::KeyNotFound { .. }
            | Self::IndexOutOfBounds { .. }
            | Self::InvalidIndex { .. }
            | Self::InvalidCast => "value",
            Self::Io { .. } => "io",
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::InvalidCharacter => Cow::Borrowed("invalid character"),
            Self::InvalidLiteral { expected } => {
                Cow::Owned(format!("invalid literal, expected {expected}"))
            }
            Self::InvalidNumber => Cow::Borrowed("invalid number"),
            Self::UnterminatedString => Cow::Borrowed("unterminated string"),
            Self::InvalidEscape => Cow::Borrowed("invalid escape"),
            Self::InvalidUnicodeEscape => Cow::Borrowed("invalid unicode escape"),
            Self::InvalidSurrogatePair => Cow::Borrowed("invalid surrogate pair"),
            Self::UnescapedControl => Cow::Borrowed("unescaped control character"),
            Self::EmptyInput => Cow::Borrowed("empty input"),
            Self::UnexpectedToken { expected, found } => Cow::Owned(format!(
                "expected {} but found {}",
                expected.name(),
                found.syntax_name(),
            )),
            Self::UnexpectedEof { expected } => Cow::Owned(format!(
                "expected {} but reached end of input",
                expected.name(),
            )),
            Self::TrailingInput => Cow::Borrowed("trailing input"),
            Self::TypeMismatch { expected, found } => Cow::Owned(format!(
                "expected {} but found {}",
                expected.name(),
                found.name(),
            )),
            Self::KeyNotFound { key } => Cow::Owned(format!("key \"{key}\" not found")),
            Self::IndexOutOfBounds { index, len } => {
                Cow::Owned(format!("index {index} out of bounds for length {len}"))
            }
            Self::InvalidIndex { key } => Cow::Owned(format!("invalid index \"{key}\"")),
            Self::InvalidCast => Cow::Borrowed("invalid cast"),
            Self::Io { message } => Cow::Owned(message.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub kind: JsonErrorKind,
    pub span: Option<Span>,
    pub location: Option<Location>,
}

impl JsonError {
    pub fn new(kind: JsonErrorKind, span: Span, source: &str) -> Self {
        Self {
            kind,
            span: Some(span),
            location: Some(Location::from_byte(source, span.start)),
        }
    }

    pub fn at(kind: JsonErrorKind, span: Span, location: Location) -> Self {
        Self {
            kind,
            span: Some(span),
            location: Some(location),
        }
    }

    pub fn value(kind: JsonErrorKind) -> Self {
        Self {
            kind,
            span: None,
            location: None,
        }
    }

    pub fn io(err: impl std::fmt::Display) -> Self {
        Self::value(JsonErrorKind::Io {
            message: err.to_string(),
        })
    }
}

impl std::error::Error for JsonError {}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match (self.location, self.span) {
            (Some(loc), Some(span)) => write!(
                f,
                "{} error at {}:{} ({}..{}): {}",
                self.kind.phase(),
                loc.line,
                loc.column,
                span.start,
                span.end,
                self.kind.message(),
            ),
            (None, Some(span)) => write!(
                f,
                "{} error at {}..{}: {}",
                self.kind.phase(),
                span.start,
                span.end,
                self.kind.message(),
            ),
            _ => write!(f, "{} error: {}", self.kind.phase(), self.kind.message()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpectedJsonType, ExpectedSyntax, JsonError, JsonErrorKind, JsonType};
    use crate::Span;
    use crate::json::token::JsonTokenKind;

    fn err(kind: JsonErrorKind, start: usize, end: usize) -> JsonError {
        let source = "x".repeat(end.max(1));
        JsonError::new(kind, Span { start, end }, &source)
    }

    #[test]
    fn display_uses_one_sentence_per_kind() {
        let cases = [
            (
                err(JsonErrorKind::InvalidCharacter, 0, 1),
                "lexical error at 1:1 (0..1): invalid character",
            ),
            (
                err(JsonErrorKind::InvalidLiteral { expected: "true" }, 0, 3),
                "lexical error at 1:1 (0..3): invalid literal, expected true",
            ),
            (
                err(JsonErrorKind::InvalidNumber, 0, 2),
                "lexical error at 1:1 (0..2): invalid number",
            ),
            (
                err(JsonErrorKind::UnterminatedString, 0, 4),
                "lexical error at 1:1 (0..4): unterminated string",
            ),
            (
                err(JsonErrorKind::InvalidEscape, 0, 3),
                "lexical error at 1:1 (0..3): invalid escape",
            ),
            (
                err(JsonErrorKind::InvalidUnicodeEscape, 0, 4),
                "lexical error at 1:1 (0..4): invalid unicode escape",
            ),
            (
                err(JsonErrorKind::InvalidSurrogatePair, 0, 8),
                "lexical error at 1:1 (0..8): invalid surrogate pair",
            ),
            (
                err(JsonErrorKind::UnescapedControl, 1, 2),
                "lexical error at 1:2 (1..2): unescaped control character",
            ),
            (
                err(JsonErrorKind::EmptyInput, 0, 0),
                "parse error at 1:1 (0..0): empty input",
            ),
            (
                err(
                    JsonErrorKind::UnexpectedToken {
                        expected: ExpectedSyntax::Token(JsonTokenKind::Colon),
                        found: JsonTokenKind::Number,
                    },
                    3,
                    4,
                ),
                "parse error at 1:4 (3..4): expected ':' but found number",
            ),
            (
                err(
                    JsonErrorKind::UnexpectedToken {
                        expected: ExpectedSyntax::Value,
                        found: JsonTokenKind::Comma,
                    },
                    2,
                    3,
                ),
                "parse error at 1:3 (2..3): expected value but found ','",
            ),
            (
                err(
                    JsonErrorKind::UnexpectedEof {
                        expected: ExpectedSyntax::Token(JsonTokenKind::RBrace),
                    },
                    1,
                    1,
                ),
                "parse error at 1:2 (1..1): expected '}' but reached end of input",
            ),
            (
                err(JsonErrorKind::TrailingInput, 5, 10),
                "parse error at 1:6 (5..10): trailing input",
            ),
            (
                JsonError::value(JsonErrorKind::TypeMismatch {
                    expected: ExpectedJsonType::ArrayOrObject,
                    found: JsonType::String,
                }),
                "value error: expected array or object but found string",
            ),
            (
                JsonError::value(JsonErrorKind::TypeMismatch {
                    expected: ExpectedJsonType::Boolean,
                    found: JsonType::Null,
                }),
                "value error: expected boolean but found null",
            ),
            (
                JsonError::value(JsonErrorKind::TypeMismatch {
                    expected: ExpectedJsonType::Number,
                    found: JsonType::String,
                }),
                "value error: expected number but found string",
            ),
            (
                JsonError::value(JsonErrorKind::TypeMismatch {
                    expected: ExpectedJsonType::Array,
                    found: JsonType::Number,
                }),
                "value error: expected array but found number",
            ),
            (
                JsonError::value(JsonErrorKind::KeyNotFound {
                    key: "name".to_string(),
                }),
                "value error: key \"name\" not found",
            ),
            (
                JsonError::value(JsonErrorKind::IndexOutOfBounds { index: 3, len: 1 }),
                "value error: index 3 out of bounds for length 1",
            ),
            (
                JsonError::value(JsonErrorKind::InvalidIndex {
                    key: "name".to_string(),
                }),
                "value error: invalid index \"name\"",
            ),
            (
                JsonError::value(JsonErrorKind::InvalidCast),
                "value error: invalid cast",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
