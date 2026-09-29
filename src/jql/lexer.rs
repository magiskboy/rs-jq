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

impl<'a> JqlLexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            index: 0,
            source: Source::new(source),
        }
    }

    fn next_token(&mut self) -> Result<JqlToken, JqlError> {
        Ok(JqlToken::new(JqlTokenKind::Stop, 0, 4))
    }

    pub fn tokenize(source: &str) -> Result<Vec<JqlToken>, JqlError> {
        let mut lexer = JqlLexer::new(source);
        let mut tokens: Vec<JqlToken> = vec![];
        loop {
            let token = lexer.next_token()?;
            if token.kind == JqlTokenKind::Stop {
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}
