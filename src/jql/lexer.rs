use crate::{
    jql::{
        error::JqlError,
        token::{JqlToken, JqlTokenKind},
    },
    source::Source,
};

#[derive(Debug, Clone)]
pub struct JqlLexer<'a> {
    pub index: usize,
    pub source: Source<'a>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
enum JqlLexerState {
    Start,
    InLBrace,
    InRBrace,
    InPipe,
    InF,
    InFa,
    InFal,
    InFals,
    InFalse,
    InT,
    InTr,
    InTru,
    InTrue,
    InN,
    InNu,
    InNul,
    InNull,
    InNumber,
    OpenString,
    InString,
    CloseString,
    InIdentifier,
    StartEqual,
    InEqual,
    StartNotEqual,
    InNotEqual,
    InGreater,
    InLess,
    InGreaterEqual,
    InLessEqual,
    StartAnd,
    InAnd,
    InOr,
    Whitespace,
    InvalidToken,
}

impl<'a> JqlLexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            index: 0,
            source: Source::new(source),
        }
    }

    fn transition_table(&self, state: &JqlLexerState, c: char) -> Option<JqlLexerState> {
        let next_state = match state {
            JqlLexerState::Start => match c {
                '"' => JqlLexerState::OpenString,
                't' => JqlLexerState::InT,
                'f' => JqlLexerState::InF,
                'n' => JqlLexerState::InN,
                '&' => JqlLexerState::StartAnd,
                '!' => JqlLexerState::StartNotEqual,
                '=' => JqlLexerState::StartEqual,
                '>' => JqlLexerState::InGreater,
                '<' => JqlLexerState::InLess,
                '|' => JqlLexerState::InPipe,
                '(' => JqlLexerState::InLBrace,
                ')' => JqlLexerState::InRBrace,
                '0'..='9' => JqlLexerState::InNumber,
                '.' | 'a'..='z' | 'A'..='Z' => JqlLexerState::InIdentifier,
                ' ' => JqlLexerState::Whitespace,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InPipe => match c {
                '|' => JqlLexerState::InOr,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNumber => match c {
                '0'..='9' => JqlLexerState::InNumber,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::OpenString | JqlLexerState::InString => match c {
                '"' => JqlLexerState::CloseString,
                _ => JqlLexerState::InString,
            },
            JqlLexerState::InT => match c {
                'r' => JqlLexerState::InTr,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InTr => match c {
                'u' => JqlLexerState::InTru,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InTru => match c {
                'e' => JqlLexerState::InTrue,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InF => match c {
                'a' => JqlLexerState::InFa,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFa => match c {
                'l' => JqlLexerState::InFal,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFal => match c {
                's' => JqlLexerState::InFals,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFals => match c {
                'e' => JqlLexerState::InFalse,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InN => match c {
                'u' => JqlLexerState::InNu,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNu => match c {
                'l' => JqlLexerState::InNul,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNul => match c {
                'l' => JqlLexerState::InNull,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InIdentifier => match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '[' | ']' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::StartEqual => JqlLexerState::InEqual,
            JqlLexerState::StartNotEqual => JqlLexerState::InNotEqual,
            JqlLexerState::InGreater => match c {
                '=' => JqlLexerState::InGreaterEqual,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InLess => match c {
                '=' => JqlLexerState::InLessEqual,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::StartAnd => JqlLexerState::InAnd,
            _ => JqlLexerState::InvalidToken,
        };

        match next_state {
            JqlLexerState::InvalidToken => None,
            _ => Some(next_state),
        }
    }

    fn accept_state(&self, state: &JqlLexerState, start: usize, end: usize) -> Option<JqlToken> {
        let kind = match state {
            JqlLexerState::InLBrace => JqlTokenKind::LParen,
            JqlLexerState::InRBrace => JqlTokenKind::RParen,
            JqlLexerState::InPipe => JqlTokenKind::Pipe,
            JqlLexerState::InFalse => JqlTokenKind::Boolean,
            JqlLexerState::InTrue => JqlTokenKind::Boolean,
            JqlLexerState::InNull => JqlTokenKind::Null,
            JqlLexerState::InNumber => JqlTokenKind::Number,
            JqlLexerState::CloseString => JqlTokenKind::String,
            JqlLexerState::InIdentifier => {
                let content = self.source.slice(start, end).unwrap();
                match Self::keyword_table(content) {
                    true => JqlTokenKind::Keyword,
                    false => JqlTokenKind::Identifier,
                }
            }
            JqlLexerState::InEqual => JqlTokenKind::EqualOp,
            JqlLexerState::InNotEqual => JqlTokenKind::NotEqualOp,
            JqlLexerState::InGreater => JqlTokenKind::GreaterOp,
            JqlLexerState::InLess => JqlTokenKind::LessOp,
            JqlLexerState::InGreaterEqual => JqlTokenKind::GreaterEqualOp,
            JqlLexerState::InLessEqual => JqlTokenKind::LessEqualOp,
            JqlLexerState::InAnd => JqlTokenKind::AndLogicalOp,
            JqlLexerState::InOr => JqlTokenKind::OrLogicalOp,
            JqlLexerState::Whitespace => JqlTokenKind::Whitespace,
            _ => JqlTokenKind::InvalidToken,
        };

        match kind {
            JqlTokenKind::InvalidToken => None,
            _ => Some(JqlToken::new(kind, start, end)),
        }
    }

    fn keyword_table(value: &str) -> bool {
        match value {
            "filter" => true,
            _ => false,
        }
    }

    fn next_token(&mut self) -> Result<JqlToken, JqlError> {
        let mut state = JqlLexerState::Start;
        let mut position = self.index;
        let mut last_accept_token: Option<JqlToken> = None;
        let mut last_accept_position = position;

        while position < self.source.len() {
            let c = self.source.char_at(position).unwrap();

            let next_state = self.transition_table(&state, c);
            if next_state.is_none() {
                break;
            }

            if let Some(token) = self.accept_state(&next_state.unwrap(), self.index, position + 1) {
                last_accept_token = Some(token);
                last_accept_position = position + 1;
            }
            state = next_state.unwrap().clone();
            position += 1;
        }

        if let Some(token) = last_accept_token {
            self.index = last_accept_position;
            if token.kind == JqlTokenKind::Whitespace {
                return self.next_token();
            }

            return Ok(token);
        }

        self.index = position + 1;
        Err(JqlError {
            kind: super::error::JqlErrorKind::GenericError,
            message: String::from("invalid token"),
        })
    }

    pub fn tokenize(source: &str) -> Result<Vec<JqlToken>, JqlError> {
        let mut lexer = JqlLexer::new(source);
        let mut tokens: Vec<JqlToken> = vec![];
        loop {
            if let Ok(token) = lexer.next_token() {
                tokens.push(token);
            } else {
                break;
            }
        }
        Ok(tokens)
    }
}
