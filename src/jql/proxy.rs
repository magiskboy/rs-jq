use std::{borrow::Cow, str::FromStr};

use crate::Structured;
use crate::jql::error::{JqlError, JqlErrorKind};
use crate::jql::reference::{Access, ArrayIndex};
use crate::jql::reference::{PropertySelection, Reference};
use crate::json::value::JsonValue;

#[derive(Debug, Clone)]
pub struct Proxy<'a> {
    data: Cow<'a, JsonValue>,
}

impl<'a> Proxy<'a> {
    pub fn new(data: &'a JsonValue) -> Self {
        Self {
            data: Cow::Borrowed(data),
        }
    }

    pub fn owned(data: JsonValue) -> Self {
        Self {
            data: Cow::Owned(data),
        }
    }

    pub fn data(&self) -> &JsonValue {
        self.data.as_ref()
    }

    fn execution_error() -> JqlError {
        JqlError::from_kind(JqlErrorKind::ExecutionError)
    }

    fn slice(value: &JsonValue, start: usize, end: usize) -> Result<JsonValue, JqlError> {
        match value {
            JsonValue::Array(items) => {
                if let Some(values) = items.get(start..end) {
                    Ok(JsonValue::array(values.to_vec()))
                } else {
                    Err(Self::execution_error())
                }
            }
            _ => Err(Self::execution_error()),
        }
    }

    fn select(value: &JsonValue, indices: Vec<usize>) -> Result<JsonValue, JqlError> {
        match value {
            JsonValue::Array(items) => {
                let values = indices
                    .iter()
                    .map(|i| items.get(*i).cloned().ok_or_else(Self::execution_error))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(JsonValue::array(values))
            }
            _ => Err(Self::execution_error()),
        }
    }

    fn select_keys(value: &JsonValue, keys: &Vec<String>) -> Result<JsonValue, JqlError> {
        match value {
            JsonValue::Array(items) => {
                let values = items
                    .iter()
                    .map(|x| match x {
                        JsonValue::Object(h) => {
                            let val = h.iter().filter_map(|(k, v)| {
                                if keys.contains(k) {
                                    return Some((k.clone(), v.clone()));
                                } else {
                                    None
                                }
                            });
                            JsonValue::object(val)
                        }
                        _ => x.clone(),
                    })
                    .collect();
                Ok(JsonValue::array(values))
            }
            JsonValue::Object(m) => {
                let val = m.iter().filter_map(|(k, v)| {
                    if keys.contains(k) {
                        return Some((k.clone(), v.clone()));
                    } else {
                        None
                    }
                });
                Ok(JsonValue::object(val))
            }
            _ => Err(Self::execution_error()),
        }
    }

    pub fn get(&self, ref_str: &str) -> Result<Proxy<'a>, JqlError> {
        let jref = Reference::from_str(ref_str)?;
        let mut current = self.data.as_ref().clone();

        for access in jref.access {
            let value = match access {
                Access::PropertyAccess {
                    property,
                    properties,
                } => {
                    // Empty name is the root ref: operate on the current value as-is.
                    let new = if property.is_empty() {
                        &current
                    } else {
                        current
                            .get(&property)
                            .map_err(|_| Self::execution_error())?
                    };
                    match properties {
                        PropertySelection::Properties(keys) => Self::select_keys(new, &keys)?,
                        _ => new.clone(),
                    }
                }
                Access::IndexAccess { index, properties } => {
                    let new = match index {
                        ArrayIndex::Index(index) => current
                            .get(&index.to_string())
                            .map_err(|_| Self::execution_error())?
                            .clone(),
                        ArrayIndex::Range { start, end } => Self::slice(&current, start, end)?,
                        ArrayIndex::MultipleIndex(indices) => Self::select(&current, indices)?,
                        _ => current.clone(),
                    };

                    match properties {
                        PropertySelection::Properties(keys) => Self::select_keys(&new, &keys)?,
                        _ => new.clone(),
                    }
                }
            };

            current = value;
        }

        Ok(Proxy {
            data: Cow::Owned(current),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Proxy;
    use crate::jql::error::JqlError;
    use crate::json::value::JsonValue;

    fn job(title: &str, description: &str, id: f32) -> JsonValue {
        JsonValue::object([
            ("title".to_string(), JsonValue::string(title.to_string())),
            (
                "description".to_string(),
                JsonValue::string(description.to_string()),
            ),
            ("id".to_string(), JsonValue::number(id)),
        ])
    }

    fn person(name: &str, age: f32, city: &str) -> JsonValue {
        JsonValue::object([
            ("name".to_string(), JsonValue::string(name.to_string())),
            ("age".to_string(), JsonValue::number(age)),
            ("city".to_string(), JsonValue::string(city.to_string())),
        ])
    }

    fn fixture() -> JsonValue {
        JsonValue::object([
            (
                "jobs".to_string(),
                JsonValue::array(vec![
                    job("Dev", "Code", 1.0),
                    job("QA", "Test", 2.0),
                    job("PM", "Plan", 3.0),
                ]),
            ),
            (
                "person".to_string(),
                JsonValue::object([
                    ("name".to_string(), JsonValue::string("Alice".to_string())),
                    ("age".to_string(), JsonValue::number(30.0)),
                    ("city".to_string(), JsonValue::string("HN".to_string())),
                ]),
            ),
            ("count".to_string(), JsonValue::number(42.0)),
            ("label".to_string(), JsonValue::string("root".to_string())),
        ])
    }

    fn proxy_get(data: &JsonValue, ref_str: &str) -> Result<JsonValue, JqlError> {
        Proxy::new(data).get(ref_str).map(|p| p.data().clone())
    }

    fn assert_get(data: &JsonValue, ref_str: &str, expected: JsonValue) {
        let actual =
            proxy_get(data, ref_str).unwrap_or_else(|_| panic!("ref {ref_str:?} should succeed"));
        assert_eq!(actual, expected, "ref {ref_str:?}");
    }

    fn assert_get_err(data: &JsonValue, ref_str: &str) {
        assert!(
            proxy_get(data, ref_str).is_err(),
            "ref {ref_str:?} should fail"
        );
    }

    #[test]
    fn get_root_ref() {
        let data = fixture();
        assert_get(&data, ".", data.clone());
    }

    #[test]
    fn get_simple_property() {
        let data = fixture();
        assert_get(
            &data,
            ".jobs",
            JsonValue::array(vec![
                job("Dev", "Code", 1.0),
                job("QA", "Test", 2.0),
                job("PM", "Plan", 3.0),
            ]),
        );
        assert_get(&data, ".count", JsonValue::number(42.0));
        assert_get(&data, ".label", JsonValue::string("root".to_string()));
    }

    #[test]
    fn get_array_index() {
        let data = fixture();
        assert_get(&data, ".jobs[1]", job("QA", "Test", 2.0));
        assert_get(&data, ".jobs[0]", job("Dev", "Code", 1.0));
        assert_get(&data, ".jobs[2]", job("PM", "Plan", 3.0));
    }

    #[test]
    fn get_array_all_is_identity() {
        let data = fixture();
        let jobs = JsonValue::array(vec![
            job("Dev", "Code", 1.0),
            job("QA", "Test", 2.0),
            job("PM", "Plan", 3.0),
        ]);
        assert_get(&data, ".jobs[]", jobs.clone());
        assert_get(&data, ".jobs", jobs);
    }

    #[test]
    fn get_array_multiple_index() {
        let data = fixture();
        assert_get(
            &data,
            ".jobs[1,2]",
            JsonValue::array(vec![job("QA", "Test", 2.0), job("PM", "Plan", 3.0)]),
        );
        assert_get(
            &data,
            ".jobs[0,2]",
            JsonValue::array(vec![job("Dev", "Code", 1.0), job("PM", "Plan", 3.0)]),
        );
    }

    #[test]
    fn get_array_range() {
        let data = fixture();
        assert_get(
            &data,
            ".jobs[1:3]",
            JsonValue::array(vec![job("QA", "Test", 2.0), job("PM", "Plan", 3.0)]),
        );
        assert_get(
            &data,
            ".jobs[0:1]",
            JsonValue::array(vec![job("Dev", "Code", 1.0)]),
        );
    }

    #[test]
    fn get_object_all_is_identity() {
        let data = fixture();
        let person = JsonValue::object([
            ("name".to_string(), JsonValue::string("Alice".to_string())),
            ("age".to_string(), JsonValue::number(30.0)),
            ("city".to_string(), JsonValue::string("HN".to_string())),
        ]);
        assert_get(&data, ".person{}", person.clone());
        assert_get(&data, ".person", person);
    }

    #[test]
    fn get_array_then_object_key_filter() {
        let data = fixture();
        assert_get(
            &data,
            ".jobs[]{title,description}",
            JsonValue::array(vec![
                JsonValue::object([
                    ("title".to_string(), JsonValue::string("Dev".to_string())),
                    (
                        "description".to_string(),
                        JsonValue::string("Code".to_string()),
                    ),
                ]),
                JsonValue::object([
                    ("title".to_string(), JsonValue::string("QA".to_string())),
                    (
                        "description".to_string(),
                        JsonValue::string("Test".to_string()),
                    ),
                ]),
                JsonValue::object([
                    ("title".to_string(), JsonValue::string("PM".to_string())),
                    (
                        "description".to_string(),
                        JsonValue::string("Plan".to_string()),
                    ),
                ]),
            ]),
        );
    }

    #[test]
    fn get_array_all_then_object_all() {
        let data = fixture();
        let jobs = JsonValue::array(vec![
            job("Dev", "Code", 1.0),
            job("QA", "Test", 2.0),
            job("PM", "Plan", 3.0),
        ]);
        assert_get(&data, ".jobs[]{}", jobs);
    }

    #[test]
    fn get_chained_properties() {
        let data = JsonValue::object([(
            "company".to_string(),
            JsonValue::object([(
                "teams".to_string(),
                JsonValue::array(vec![JsonValue::object([(
                    "name".to_string(),
                    JsonValue::string("platform".to_string()),
                )])]),
            )]),
        )]);

        assert_get(
            &data,
            ".company.teams[0].name",
            JsonValue::string("platform".to_string()),
        );
    }

    #[test]
    fn get_invalid_reference_string() {
        let data = fixture();
        assert_get_err(&data, "jobs");
        assert_get_err(&data, ".jobs[");
        assert_get_err(&data, ".jobs[1");
        assert_get_err(&data, ".jobs[1:2:3]");
    }

    #[test]
    fn get_missing_property() {
        let data = fixture();
        assert_get_err(&data, ".missing");
        assert_get_err(&data, ".jobs.missing");
    }

    #[test]
    fn get_array_index_out_of_bounds() {
        let data = fixture();
        assert_get_err(&data, ".jobs[9]");
        assert_get_err(&data, ".jobs[1,9]");
    }

    #[test]
    fn get_array_ops_on_non_array() {
        let data = fixture();
        assert_get_err(&data, ".count[0]");
        assert_get_err(&data, ".count[0:1]");
        assert_get_err(&data, ".count[0,1]");
        assert_get_err(&data, ".label[0]");
    }

    #[test]
    fn get_object_key_filter_on_object() {
        let data = fixture();
        let result = Proxy::new(&data)
            .get(".person{name,age,city}")
            .expect("should resolve reference");

        assert_eq!(result.data(), &person("Alice", 30.0, "HN"))
    }

    #[test]
    fn get_array_range_out_of_bounds() {
        let data = fixture();
        assert_get_err(&data, ".jobs[2:9]");
    }

    #[test]
    fn get_returns_owned_proxy_data() {
        let data = fixture();
        let proxy = Proxy::new(&data);
        let result = proxy.get(".jobs[1]").expect("should resolve reference");
        assert_eq!(result.data(), &job("QA", "Test", 2.0));
    }
}
