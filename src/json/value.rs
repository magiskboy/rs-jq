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
