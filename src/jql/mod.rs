use crate::jql::{error::JqlError, lexer::JqlLexer, token::JqlToken};

pub(crate) mod error;
pub(crate) mod lexer;
pub(crate) mod token;

//TODO: complete with JqlAst
pub fn jql_parse(source: &str) -> Result<Vec<JqlToken>, JqlError> {
    JqlLexer::tokenize(source)
}
