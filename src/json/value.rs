use std::{collections::HashMap, fmt::Display};

use crate::json::{JsonDumpOptions, json_dumps};

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
        let opts = JsonDumpOptions::default();
        json_dumps(f, self, &opts)
    }
}

impl JsonValue {
    pub fn null() -> JsonValue {
        JsonValue::Null
    }

    pub fn boolean(value: bool) -> JsonValue {
        match value {
            true => JsonValue::True,
            false => JsonValue::False,
        }
    }

    pub fn number(value: f32) -> JsonValue {
        JsonValue::Number(value)
    }

    pub fn string(value: String) -> JsonValue {
        JsonValue::String(value)
    }

    pub fn array(items: Vec<JsonValue>) -> JsonValue {
        JsonValue::Array(items)
    }

    pub fn object(items: impl IntoIterator<Item = (String, JsonValue)>) -> JsonValue {
        let object = items.into_iter().collect();
        JsonValue::Object(object)
    }
}
