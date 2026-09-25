use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    String(String),
    Number(f32),
    Null,
    True,
    False,
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl Display for JsonValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonValue::Null => write!(f, "null"),
            JsonValue::True => write!(f, "true"),
            JsonValue::False => write!(f, "false"),
            JsonValue::Number(n) => write!(f, "{}", n),
            JsonValue::String(st) => write!(f, "\"{}\"", st),
            JsonValue::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",{}", item)?;
                    } else {
                        write!(f, "{}", item)?;
                    }
                }
                write!(f, "]")
            }
            JsonValue::Object(value) => {
                write!(f, "{{")?;
                for (i, (k, v)) in value.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",\"{}\":{}", k, v)?;
                    } else {
                        write!(f, "\"{}\":{}", k, v)?;
                    }
                }
                write!(f, "}}")
            }
        }?;
        Ok(())
    }
}
