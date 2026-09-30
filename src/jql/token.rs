use crate::source::Span;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JqlTokenKind {
    LParen,
    RParen,
    Pipe,

    // data types
    Number,
    String,
    Boolean,
    Null,

    Identifier,
    EqualOp,
    NotEqualOp,
    GreaterOp,
    LessOp,
    GreaterEqualOp,
    LessEqualOp,
    AndLogicalOp,
    OrLogicalOp,
    Keyword,

    // Special tokens
    Stop,
    InvalidToken,
    Whitespace,
}

impl std::fmt::Display for JqlTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LParen => write!(f, "LParen"),
            Self::RParen => write!(f, "RParen"),
            Self::Pipe => write!(f, "Pipe"),
            Self::Number => write!(f, "Number"),
            Self::String => write!(f, "String"),
            Self::Boolean => write!(f, "Boolean"),
            Self::Null => write!(f, "Null"),
            Self::Identifier => write!(f, "Identifier"),
            Self::EqualOp => write!(f, "Equal"),
            Self::NotEqualOp => write!(f, "NotEqualOp"),
            Self::GreaterOp => write!(f, "GreaterOp"),
            Self::GreaterEqualOp => write!(f, "GreaterEqualOp"),
            Self::LessOp => write!(f, "LessOp"),
            Self::LessEqualOp => write!(f, "LessEqualOp"),
            Self::AndLogicalOp => write!(f, "AndLogicalOp"),
            Self::OrLogicalOp => write!(f, "OrLogicalOp"),
            Self::Keyword => write!(f, "Keyword"),
            Self::Stop => write!(f, "Stop"),
            Self::Whitespace => write!(f, "Whitespace"),
            Self::InvalidToken => write!(f, "InvalidToken"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JqlToken {
    pub kind: JqlTokenKind,
    pub span: Span,
}

impl JqlToken {
    pub fn new(kind: JqlTokenKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span { start, end },
        }
    }

    pub fn display<'a>(&'a self, source: &'a str) -> JqlTokenDisplay<'a> {
        JqlTokenDisplay {
            token: self,
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JqlTokenDisplay<'a> {
    token: &'a JqlToken,
    source: &'a str,
}

impl<'a> std::fmt::Display for JqlTokenDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = self
            .source
            .get(self.token.span.start..self.token.span.end)
            .unwrap_or("");
        write!(
            f,
            "<kind={}, start={}, end={}, content={} />",
            self.token.kind, self.token.span.start, self.token.span.end, content
        )
    }
}
