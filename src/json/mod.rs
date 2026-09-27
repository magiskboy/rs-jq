use crate::json::{error::JsonParserError, parser::JsonParser, value::JsonValue};

pub mod error;
pub(crate) mod escape;
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod token;
pub mod value;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JsonDumpOptions {
    indent: i32,
    tab: bool,
}

pub fn json_load(source: &str) -> Result<JsonValue, JsonParserError> {
    JsonParser::parse(source)
}

pub fn json_dump(
    writer: &mut impl std::io::Write,
    value: &JsonValue,
    opts: &JsonDumpOptions,
) -> Result<(), std::io::Error> {
    //TODO: need circle ref check before dump
    match value {
        JsonValue::Null => write!(writer, "null"),
        JsonValue::True => write!(writer, "true"),
        JsonValue::False => write!(writer, "false"),
        JsonValue::Number(n) => write!(writer, "{}", n),
        JsonValue::String(st) => write!(writer, "\"{}\"", st),
        JsonValue::Array(items) => {
            write!(writer, "[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                json_dump(writer, item, opts)?
            }
            write!(writer, "]")
        }
        JsonValue::Object(value) => {
            write!(writer, "{{")?;
            for (i, (k, v)) in value.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }

                write!(writer, "\"{}\":", k)?;
                json_dump(writer, v, opts)?
            }
            write!(writer, "}}")
        }
    }
}

pub fn json_dumps(
    writer: &mut impl std::fmt::Write,
    value: &JsonValue,
    opts: &JsonDumpOptions,
) -> std::fmt::Result {
    let mut bytes = Vec::new();
    json_dump(&mut bytes, value, opts).map_err(|_| std::fmt::Error)?;
    let string = String::from_utf8(bytes).map_err(|_| std::fmt::Error)?;
    write!(writer, "{}", string)
}
