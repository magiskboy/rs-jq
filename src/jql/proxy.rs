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

    pub fn len(&self) -> Result<JsonValue, JqlError> {
        self.data
            .as_ref()
            .len()
            .and_then(|l| Ok(JsonValue::number(l as f32)))
            .map_err(|_| Self::execution_error())
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
    use crate::jql::fixture::{at, sample};
    use crate::json::value::JsonValue;

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

    fn project_keys(value: &JsonValue, keys: &[&str]) -> JsonValue {
        match value {
            JsonValue::Object(map) => JsonValue::object(
                keys.iter()
                    .filter_map(|k| map.get(*k).map(|v| ((*k).to_string(), v.clone()))),
            ),
            JsonValue::Array(items) => {
                JsonValue::array(items.iter().map(|item| project_keys(item, keys)).collect())
            }
            other => other.clone(),
        }
    }

    #[test]
    fn get_root_ref() {
        let data = sample();
        assert_get(&data, ".", data.clone());
    }

    #[test]
    fn get_simple_property() {
        let data = sample();
        assert_get(&data, ".jobs", at(".jobs"));
        assert_get(&data, ".count", JsonValue::number(42.0));
        assert_get(&data, ".label", JsonValue::string("root".to_string()));
        assert_get(&data, ".name", JsonValue::string("Alice".to_string()));
        assert_get(&data, ".flag", JsonValue::True);
        assert_get(&data, ".house", JsonValue::Null);
    }

    #[test]
    fn get_array_index() {
        let data = sample();
        assert_get(&data, ".jobs[1]", at(".jobs[1]"));
        assert_get(&data, ".jobs[0]", at(".jobs[0]"));
        assert_get(&data, ".jobs[2]", at(".jobs[2]"));
        assert_get(&data, ".jobs[3]", at(".jobs[3]"));
        assert_get(
            &data,
            ".people[0].name",
            JsonValue::string("Bob".to_string()),
        );
    }

    #[test]
    fn get_array_all_is_identity() {
        let data = sample();
        let jobs = at(".jobs");
        assert_get(&data, ".jobs[]", jobs.clone());
        assert_get(&data, ".jobs", jobs);
        assert_get(&data, ".people[]", at(".people"));
    }

    #[test]
    fn get_array_multiple_index() {
        let data = sample();
        assert_get(
            &data,
            ".jobs[1,2]",
            JsonValue::array(vec![at(".jobs[1]"), at(".jobs[2]")]),
        );
        assert_get(
            &data,
            ".jobs[0,2]",
            JsonValue::array(vec![at(".jobs[0]"), at(".jobs[2]")]),
        );
        assert_get(
            &data,
            ".jobs[0,1,3]",
            JsonValue::array(vec![at(".jobs[0]"), at(".jobs[1]"), at(".jobs[3]")]),
        );
        assert_get(
            &data,
            ".matrix[0,2]",
            JsonValue::array(vec![at(".matrix[0]"), at(".matrix[2]")]),
        );
    }

    #[test]
    fn get_array_range() {
        let data = sample();
        assert_get(
            &data,
            ".jobs[1:3]",
            JsonValue::array(vec![at(".jobs[1]"), at(".jobs[2]")]),
        );
        assert_get(&data, ".jobs[0:1]", JsonValue::array(vec![at(".jobs[0]")]));
        assert_get(
            &data,
            ".candidates[1:4]",
            JsonValue::array(vec![
                at(".candidates[1]"),
                at(".candidates[2]"),
                at(".candidates[3]"),
            ]),
        );
        assert_get(
            &data,
            ".matrix[0:2]",
            JsonValue::array(vec![at(".matrix[0]"), at(".matrix[1]")]),
        );
    }

    #[test]
    fn get_object_all_is_identity() {
        let data = sample();
        let person = at(".person");
        assert_get(&data, ".person{}", person.clone());
        assert_get(&data, ".person", person);
        assert_get(&data, ".company.meta{}", at(".company.meta"));
    }

    #[test]
    fn get_array_then_object_key_filter() {
        let data = sample();
        assert_get(
            &data,
            ".jobs[]{title,description}",
            project_keys(&at(".jobs"), &["title", "description"]),
        );
        assert_get(
            &data,
            ".jobs[]{title,id}",
            project_keys(&at(".jobs"), &["title", "id"]),
        );
        assert_get(
            &data,
            ".people[]{name,vip}",
            project_keys(&at(".people"), &["name", "vip"]),
        );
    }

    #[test]
    fn get_array_all_then_object_all() {
        let data = sample();
        assert_get(&data, ".jobs[]{}", at(".jobs"));
        assert_get(&data, ".people[]{}", at(".people"));
    }

    #[test]
    fn get_chained_properties() {
        let data = sample();
        assert_get(
            &data,
            ".company.teams[0].name",
            JsonValue::string("platform".to_string()),
        );
        assert_get(&data, ".company.teams[1].size", JsonValue::number(5.0));
        assert_get(
            &data,
            ".company.meta.region",
            JsonValue::string("APAC".to_string()),
        );
        assert_get(
            &data,
            ".rows[1][0].name",
            JsonValue::string("b".to_string()),
        );
        assert_get(&data, ".nested[1][1][0]", JsonValue::number(3.0));
        assert_get(&data, ".matrix[2][0]", JsonValue::number(7.0));
    }

    #[test]
    fn get_nested_index_selection_and_projection() {
        let data = sample();
        assert_get(
            &data,
            ".jobs[0,2]{title,active}",
            project_keys(
                &JsonValue::array(vec![at(".jobs[0]"), at(".jobs[2]")]),
                &["title", "active"],
            ),
        );
        assert_get(
            &data,
            ".jobs[1:3]{id,level}",
            project_keys(
                &JsonValue::array(vec![at(".jobs[1]"), at(".jobs[2]")]),
                &["id", "level"],
            ),
        );
        assert_get(
            &data,
            ".person{name,age}",
            project_keys(&at(".person"), &["name", "age"]),
        );
        assert_get(
            &data,
            ".eq{xs,ys,obj}",
            project_keys(&at(".eq"), &["xs", "ys", "obj"]),
        );
    }

    #[test]
    fn get_invalid_reference_string() {
        let data = sample();
        assert_get_err(&data, "jobs");
        assert_get_err(&data, ".jobs[");
        assert_get_err(&data, ".jobs[1");
        assert_get_err(&data, ".jobs[1:2:3]");
    }

    #[test]
    fn get_missing_property() {
        let data = sample();
        assert_get_err(&data, ".missing");
        assert_get_err(&data, ".jobs.missing");
        assert_get_err(&data, ".company.missing");
    }

    #[test]
    fn get_array_index_out_of_bounds() {
        let data = sample();
        assert_get_err(&data, ".jobs[9]");
        assert_get_err(&data, ".jobs[1,9]");
        assert_get_err(&data, ".matrix[3]");
    }

    #[test]
    fn get_array_ops_on_non_array() {
        let data = sample();
        assert_get_err(&data, ".count[0]");
        assert_get_err(&data, ".count[0:1]");
        assert_get_err(&data, ".count[0,1]");
        assert_get_err(&data, ".label[0]");
        assert_get_err(&data, ".person[0]");
    }

    #[test]
    fn get_object_key_filter_on_object() {
        let data = sample();
        let result = Proxy::new(&data)
            .get(".person{name,age,city}")
            .expect("should resolve reference");

        assert_eq!(
            result.data(),
            &project_keys(&at(".person"), &["name", "age", "city"])
        );
    }

    #[test]
    fn get_array_range_out_of_bounds() {
        let data = sample();
        assert_get_err(&data, ".jobs[2:9]");
        assert_get_err(&data, ".candidates[0:9]");
    }

    #[test]
    fn get_returns_owned_proxy_data() {
        let data = sample();
        let proxy = Proxy::new(&data);
        let result = proxy.get(".jobs[1]").expect("should resolve reference");
        assert_eq!(result.data(), &at(".jobs[1]"));
    }

    #[test]
    fn get_empty_containers() {
        let data = sample();
        assert_get(&data, ".empty.arr", JsonValue::array(vec![]));
        assert_get(&data, ".empty.obj", JsonValue::object([]));
        assert_get(&data, ".empty.arr[]", JsonValue::array(vec![]));
    }
}
