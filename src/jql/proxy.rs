use std::{borrow::Cow, str::FromStr};

use crate::Structured;
use crate::jql::reference::ArrayExtra;
use crate::jql::reference::ObjectExtra;
use crate::jql::reference::Reference;
use crate::json::value::JsonValue;

#[derive(Debug, Clone)]
pub enum ProxyError {
    GenericError,
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::GenericError => write!(f, "GenericError"),
        }
    }
}

impl std::error::Error for ProxyError {}

#[derive(Debug, Clone)]
pub struct Proxy<'a> {
    data: Cow<'a, JsonValue>,
}

impl<'a> std::fmt::Display for Proxy<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Proxy")
    }
}

impl<'a> Proxy<'a> {
    pub fn new(data: &'a JsonValue) -> Self {
        Self {
            data: Cow::Borrowed(data),
        }
    }

    pub fn data(&self) -> &JsonValue {
        self.data.as_ref()
    }

    fn slice(value: &JsonValue, start: usize, end: usize) -> Result<JsonValue, ProxyError> {
        match value {
            JsonValue::Array(items) => {
                if let Some(values) = items.get(start..end) {
                    Ok(JsonValue::array(values.to_vec()))
                } else {
                    Err(ProxyError::GenericError)
                }
            }
            _ => Err(ProxyError::GenericError),
        }
    }

    fn select(value: &JsonValue, indices: Vec<usize>) -> Result<JsonValue, ProxyError> {
        match value {
            JsonValue::Array(items) => {
                let values = indices
                    .iter()
                    .map(|i| items.get(*i).cloned().ok_or(ProxyError::GenericError))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(JsonValue::array(values))
            }
            _ => Err(ProxyError::GenericError),
        }
    }

    fn select_keys(value: &JsonValue, keys: &Vec<String>) -> Result<JsonValue, ProxyError> {
        match value {
            JsonValue::Array(items) => {
                //
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
            _ => Err(ProxyError::GenericError),
        }
    }

    pub fn get(&self, ref_str: &str) -> Result<Proxy<'a>, ProxyError> {
        let jref = Reference::from_str(ref_str)?;
        let mut current = self.data.as_ref().clone();

        for prop in jref.properties {
            let value = current
                .get(&prop.name)
                .map_err(|_| ProxyError::GenericError)?
                .clone();

            current = match prop.array_extra {
                ArrayExtra::Index(idx) => value
                    .get(&idx.to_string())
                    .map_err(|_| ProxyError::GenericError)?
                    .clone(),
                ArrayExtra::Range { start, end } => Self::slice(&value, start, end)?,
                ArrayExtra::MultipleIndex(indices) => Self::select(&value, indices)?,
                _ => value,
            };

            current = match prop.object_extra {
                ObjectExtra::MultipleProperties(names) => {
                    Self::select_keys(&current, &names).map_err(|_| ProxyError::GenericError)?
                }
                _ => current,
            };
        }

        Ok(Proxy {
            data: Cow::Owned(current),
        })
    }
}
