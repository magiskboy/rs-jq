use crate::jql::{ast::JqlAstNode, error::JqlError, lexer::JqlLexer, parser::JqlParser};

pub(crate) mod ast;
pub mod engine;
pub(crate) mod error;
pub mod funcs;
pub(crate) mod lexer;
pub(crate) mod parser;
pub mod proxy;
pub(crate) mod reference;
pub(crate) mod token;

pub fn jql_parse<'a>(source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
    let tokens = JqlLexer::tokenize(source)?;
    JqlParser::parse(&tokens, source)
}
