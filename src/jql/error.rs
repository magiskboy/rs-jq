use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use crate::jql::token::JqlTokenKind;
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpectedSyntax {
    BinaryOperator,
    Token(JqlTokenKind),
}

impl ExpectedSyntax {
    fn name(&self) -> Cow<'static, str> {
        match self {
            Self::BinaryOperator => Cow::Borrowed("binary operator"),
            Self::Token(kind) => Cow::Borrowed(kind.syntax_name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JqlErrorKind {
    InvalidToken,
    MissingOperand,
    UnclosedParen,
    InvalidExpression,
    UnexpectedToken {
        expected: ExpectedSyntax,
        found: JqlTokenKind,
    },
    InvalidAccess,
}

impl JqlErrorKind {
    fn phase(&self) -> &'static str {
        match self {
            Self::InvalidToken => "lexical",
            Self::MissingOperand
            | Self::UnclosedParen
            | Self::InvalidExpression
            | Self::UnexpectedToken { .. } => "parse",
            Self::InvalidAccess { .. } => "execute",
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::InvalidToken => Cow::Borrowed("invalid token"),
            Self::MissingOperand => Cow::Borrowed("expected an operand"),
            Self::UnclosedParen => Cow::Borrowed("expected a closed parenthesis"),
            Self::InvalidExpression => Cow::Borrowed("unterminated token"),
            Self::UnexpectedToken { expected, found } => Cow::Owned(format!(
                "expected {} but found {}",
                expected.name(),
                found.syntax_name(),
            )),
            Self::InvalidAccess => Cow::Borrowed("invalid reference"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JqlError {
    pub kind: JqlErrorKind,
    pub span: Option<Span>,
}

impl JqlError {
    pub fn new(kind: JqlErrorKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }

    pub fn from_kind(kind: JqlErrorKind) -> Self {
        Self { kind, span: None }
    }
}

impl std::error::Error for JqlError {}

impl Display for JqlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.span {
            Some(span) => write!(
                f,
                "{} error at {}..{}: {}",
                self.kind.phase(),
                span.start,
                span.end,
                self.kind.message(),
            ),
            None => write!(f, "{} error: {}", self.kind.phase(), self.kind.message()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExpectedSyntax, JqlError, JqlErrorKind};
    use crate::jql::token::JqlTokenKind;
    use crate::source::Span;

    fn err(kind: JqlErrorKind, start: usize, end: usize) -> JqlError {
        JqlError::new(kind, Span { start, end })
    }

    #[test]
    fn display_uses_one_sentence_per_kind() {
        let cases = [
            (
                err(JqlErrorKind::InvalidToken, 0, 1),
                "lexical error at 0..1: invalid token",
            ),
            (
                JqlError::from_kind(JqlErrorKind::MissingOperand),
                "parse error: expected an operand",
            ),
            (
                err(JqlErrorKind::UnclosedParen, 2, 8),
                "parse error at 2..8: expected a closed parenthesis",
            ),
            (
                JqlError::from_kind(JqlErrorKind::InvalidExpression),
                "parse error: unterminated token",
            ),
            (
                err(
                    JqlErrorKind::UnexpectedToken {
                        expected: ExpectedSyntax::Token(JqlTokenKind::Number),
                        found: JqlTokenKind::String,
                    },
                    0,
                    3,
                ),
                "parse error at 0..3: expected number but found string",
            ),
            (
                JqlError::from_kind(JqlErrorKind::UnexpectedToken {
                    expected: ExpectedSyntax::BinaryOperator,
                    found: JqlTokenKind::Pipe,
                }),
                "parse error: expected binary operator but found pipe",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
