use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonTokenKind {
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    String,
    Number,
    True,
    False,
    Null,
    Colon,
    Comma,
    Whitespace,
    InvalidToken,
    Stop,
}

impl Display for JsonTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonTokenKind::LBrace => write!(f, "LBrace"),
            JsonTokenKind::RBrace => write!(f, "RBrace"),
            JsonTokenKind::LBracket => write!(f, "LBracket"),
            JsonTokenKind::RBracket => write!(f, "RBracket"),
            JsonTokenKind::String => write!(f, "String"),
            JsonTokenKind::Number => write!(f, "Number"),
            JsonTokenKind::True => write!(f, "True"),
            JsonTokenKind::False => write!(f, "False"),
            JsonTokenKind::Null => write!(f, "Null"),
            JsonTokenKind::Colon => write!(f, "Colon"),
            JsonTokenKind::Comma => write!(f, "Comma"),
            JsonTokenKind::Whitespace => write!(f, "Whitespace"),
            JsonTokenKind::InvalidToken => write!(f, "InvalidToken"),
            JsonTokenKind::Stop => write!(f, "Stop"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonToken {
    pub kind: JsonTokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct JsonTokenDisplay<'a> {
    pub source: &'a str,
    pub token: &'a JsonToken,
}

impl<'a> Display for JsonTokenDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = self
            .source
            .get(self.token.span.start..self.token.span.end)
            .unwrap_or("");
        write!(
            f,
            "<kind={}, start={}, end={}, content={}",
            self.token.kind, self.token.span.start, self.token.span.end, content
        )
    }
}

impl JsonToken {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn display<'a>(&'a self, source: &'a str) -> JsonTokenDisplay<'a> {
        JsonTokenDisplay {
            source,
            token: self,
        }
    }
}
