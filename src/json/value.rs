use std::{collections::HashMap, fmt::Display};

use crate::json::{
    JsonDumpOptions,
    error::{ExpectedJsonType, JsonError, JsonErrorKind, JsonType},
    json_dumps,
};

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

    pub fn get(&self, key: &str) -> Result<&JsonValue, JsonError> {
        match self {
            JsonValue::Array(items) => {
                let index = parse_index(key)?;
                items.get(index).ok_or_else(|| {
                    JsonError::value(JsonErrorKind::IndexOutOfBounds {
                        index,
                        len: items.len(),
                    })
                })
            }
            JsonValue::Object(object) => object.get(key).ok_or_else(|| {
                JsonError::value(JsonErrorKind::KeyNotFound {
                    key: key.to_string(),
                })
            }),
            other => Err(type_mismatch(other, ExpectedJsonType::ArrayOrObject)),
        }
    }

    pub fn insert(&mut self, key: &str, element: JsonValue) -> Result<(), JsonError> {
        match self {
            JsonValue::Object(object) => {
                let _ = object.insert(key.to_string(), element);
                Ok(())
            }
            JsonValue::Array(items) => {
                let index = parse_index(key)?;
                if index > items.len() {
                    return Err(JsonError::value(JsonErrorKind::IndexOutOfBounds {
                        index,
                        len: items.len(),
                    }));
                }
                items.insert(index, element);
                Ok(())
            }
            other => Err(type_mismatch(other, ExpectedJsonType::ArrayOrObject)),
        }
    }

    pub fn len(&self) -> Result<usize, JsonError> {
        match self {
            JsonValue::Object(elements) => Ok(elements.len()),
            JsonValue::Array(items) => Ok(items.len()),
            other => Err(type_mismatch(other, ExpectedJsonType::ArrayOrObject)),
        }
    }

    pub fn push(&mut self, element: JsonValue) -> Result<(), JsonError> {
        match self {
            JsonValue::Array(items) => {
                items.push(element);
                Ok(())
            }
            other => Err(type_mismatch(other, ExpectedJsonType::Array)),
        }
    }
}

fn json_type(value: &JsonValue) -> JsonType {
    match value {
        JsonValue::Null => JsonType::Null,
        JsonValue::True | JsonValue::False => JsonType::Boolean,
        JsonValue::Number(_) => JsonType::Number,
        JsonValue::String(_) => JsonType::String,
        JsonValue::Array(_) => JsonType::Array,
        JsonValue::Object(_) => JsonType::Object,
    }
}

fn type_mismatch(value: &JsonValue, expected: ExpectedJsonType) -> JsonError {
    JsonError::value(JsonErrorKind::TypeMismatch {
        expected,
        found: json_type(value),
    })
}

fn parse_index(key: &str) -> Result<usize, JsonError> {
    key.parse().map_err(|_| {
        JsonError::value(JsonErrorKind::InvalidIndex {
            key: key.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::JsonValue;
    use crate::json::error::{ExpectedJsonType, JsonErrorKind, JsonType};

    #[test]
    fn get_reports_missing_key_bad_index_and_type() {
        let object = JsonValue::object([("name".to_string(), JsonValue::string("a".to_string()))]);
        assert_eq!(
            object.get("name").unwrap(),
            &JsonValue::string("a".to_string())
        );
        let err = object.get("missing").unwrap_err();
        assert_eq!(err.span, None);
        assert_eq!(
            err.kind,
            JsonErrorKind::KeyNotFound {
                key: "missing".to_string(),
            }
        );

        let array = JsonValue::array(vec![JsonValue::number(1.0)]);
        assert_eq!(array.get("0").unwrap(), &JsonValue::number(1.0));
        assert_eq!(
            array.get("2").unwrap_err().kind,
            JsonErrorKind::IndexOutOfBounds { index: 2, len: 1 }
        );
        assert_eq!(
            array.get("name").unwrap_err().kind,
            JsonErrorKind::InvalidIndex {
                key: "name".to_string(),
            }
        );

        assert_eq!(
            JsonValue::Null.get("0").unwrap_err().kind,
            JsonErrorKind::TypeMismatch {
                expected: ExpectedJsonType::ArrayOrObject,
                found: JsonType::Null,
            }
        );
    }

    #[test]
    fn insert_adds_object_keys_and_shifts_array_items() {
        let mut object = JsonValue::object([]);
        object
            .insert("name", JsonValue::string("a".to_string()))
            .unwrap();
        object
            .insert("name", JsonValue::string("b".to_string()))
            .unwrap();
        assert_eq!(
            object.get("name").unwrap(),
            &JsonValue::string("b".to_string())
        );

        let mut array = JsonValue::array(vec![JsonValue::number(1.0)]);
        array.insert("1", JsonValue::number(2.0)).unwrap();
        array.insert("0", JsonValue::number(0.0)).unwrap();
        assert_eq!(array.len().unwrap(), 3);
        assert_eq!(array.get("0").unwrap(), &JsonValue::number(0.0));
        assert_eq!(array.get("2").unwrap(), &JsonValue::number(2.0));

        assert_eq!(
            array.insert("9", JsonValue::Null).unwrap_err().kind,
            JsonErrorKind::IndexOutOfBounds { index: 9, len: 3 }
        );
        assert_eq!(
            array.insert("name", JsonValue::Null).unwrap_err().kind,
            JsonErrorKind::InvalidIndex {
                key: "name".to_string(),
            }
        );
        assert_eq!(
            JsonValue::True
                .insert("0", JsonValue::Null)
                .unwrap_err()
                .kind,
            JsonErrorKind::TypeMismatch {
                expected: ExpectedJsonType::ArrayOrObject,
                found: JsonType::Boolean,
            }
        );
    }

    #[test]
    fn len_and_push_require_the_matching_container() {
        let array = JsonValue::array(vec![JsonValue::Null, JsonValue::True]);
        assert_eq!(array.len().unwrap(), 2);
        assert_eq!(JsonValue::object([]).len().unwrap(), 0);
        assert_eq!(
            JsonValue::number(1.0).len().unwrap_err().kind,
            JsonErrorKind::TypeMismatch {
                expected: ExpectedJsonType::ArrayOrObject,
                found: JsonType::Number,
            }
        );

        let mut array = JsonValue::array(vec![]);
        array.push(JsonValue::False).unwrap();
        assert_eq!(array.len().unwrap(), 1);

        assert_eq!(
            JsonValue::object([])
                .push(JsonValue::Null)
                .unwrap_err()
                .kind,
            JsonErrorKind::TypeMismatch {
                expected: ExpectedJsonType::Array,
                found: JsonType::Object,
            }
        );
    }
}
