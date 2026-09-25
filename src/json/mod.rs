use crate::json::{error::JsonParserError, parser::JsonParser, value::JsonValue};

pub mod error;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod token;
pub(crate) mod escape;
pub mod value;

pub fn json_load(source: &str) -> Result<JsonValue, JsonParserError> {
    JsonParser::parse(source)
}

pub fn json_dumps(value: &JsonValue) -> Result<String, JsonParserError> {
    //TODO: need circle ref check before dump
    Ok(value.to_string())
}
