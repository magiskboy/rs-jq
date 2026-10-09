use std::{fmt::Debug, str::FromStr};

use crate::jql::error::{JqlError, JqlErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySelection {
    Properties(Vec<String>),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    Range { start: usize, end: usize },
    Index(usize),
    MultipleIndex(Vec<usize>),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    IndexAccess {
        index: ArrayIndex,
        properties: PropertySelection,
    },
    PropertyAccess {
        property: String,
        properties: PropertySelection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub access: Vec<Access>,
}

#[derive(Debug, Clone)]
enum ParseReferenceState {
    StartProperty,
    InProperty,
    StartArrayIndex,
    InArrayIndex,
    InArraySlice,
    InArraySliceColon,
    InMultipleIndex,
    InMultipleIndexComma,
    CloseArrayIndex,
    StartPropertySelection,
    InPropertySelection,
    InPropertyComma,
    ClosePropertySelection,
    InvalidCharacter,
}

impl Reference {
    fn accept_state(
        state: &ParseReferenceState,
        start: usize,
        end: usize,
        source: &str,
        last_access: &Access,
    ) -> Result<Access, JqlError> {
        match state {
            ParseReferenceState::InProperty => {
                let property = source
                    .get(start..end)
                    .ok_or_else(Self::invalid_access)?
                    .to_string();
                Ok(Access::PropertyAccess {
                    property,
                    properties: PropertySelection::All,
                })
            }

            ParseReferenceState::CloseArrayIndex => {
                let s = source
                    .get(start..end - 1)
                    .ok_or_else(Self::invalid_access)?;

                if s.is_empty() {
                    return Ok(Access::IndexAccess {
                        index: ArrayIndex::All,
                        properties: PropertySelection::All,
                    });
                }

                if let Ok(index) = Self::parse_usize(s) {
                    return Ok(Access::IndexAccess {
                        index: ArrayIndex::Index(index),
                        properties: PropertySelection::All,
                    });
                }

                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 2 {
                    let start = Self::parse_usize(&parts[0])?;
                    let end = Self::parse_usize(&parts[1])?;
                    return Ok(Access::IndexAccess {
                        index: ArrayIndex::Range { start, end },
                        properties: PropertySelection::All,
                    });
                }

                let indices = s
                    .split(',')
                    .map(Self::parse_usize)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Access::IndexAccess {
                    index: ArrayIndex::MultipleIndex(indices),
                    properties: PropertySelection::All,
                })
            }

            ParseReferenceState::ClosePropertySelection => {
                let s = source
                    .get(start..end - 1)
                    .ok_or_else(Self::invalid_access)?;
                if let Some(lbrace) = s.find('{') {
                    let keys: Vec<String> = s
                        .get(lbrace + 1..)
                        .ok_or_else(Self::invalid_access)?
                        .split(",")
                        .map(|x| x.to_string())
                        .filter(|x| !x.is_empty())
                        .collect();

                    let selected_props = if keys.is_empty() {
                        PropertySelection::All
                    } else {
                        PropertySelection::Properties(keys)
                    };
                    return Ok(match last_access {
                        Access::IndexAccess {
                            index,
                            properties: _,
                        } => Access::IndexAccess {
                            index: index.clone(),
                            properties: selected_props,
                        },
                        Access::PropertyAccess {
                            property,
                            properties: _,
                        } => Access::PropertyAccess {
                            property: property.clone(),
                            properties: selected_props,
                        },
                    });
                }
                Err(Self::invalid_access())
            }
            _ => Err(Self::invalid_access()),
        }
    }

    fn transition_table(state: &ParseReferenceState, c: char) -> Option<ParseReferenceState> {
        let next_state = match state {
            ParseReferenceState::StartProperty => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InProperty,
                '{' => ParseReferenceState::StartPropertySelection,
                '[' => ParseReferenceState::StartArrayIndex,
                '.' => ParseReferenceState::StartProperty,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InProperty => match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => ParseReferenceState::InProperty,
                '.' => ParseReferenceState::StartProperty,
                '[' => ParseReferenceState::StartArrayIndex,
                '{' => ParseReferenceState::StartPropertySelection,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::StartArrayIndex => match c {
                '0'..='9' => ParseReferenceState::InArrayIndex,
                ']' => ParseReferenceState::CloseArrayIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InArrayIndex => match c {
                '0'..='9' => ParseReferenceState::InArrayIndex,
                ']' => ParseReferenceState::CloseArrayIndex,
                ',' => ParseReferenceState::InMultipleIndexComma,
                ':' => ParseReferenceState::InArraySliceColon,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InMultipleIndexComma => match c {
                '0'..='9' => ParseReferenceState::InMultipleIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InMultipleIndex => match c {
                '0'..='9' => ParseReferenceState::InMultipleIndex,
                ',' => ParseReferenceState::InMultipleIndexComma,
                ']' => ParseReferenceState::CloseArrayIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InArraySliceColon => match c {
                '0'..='9' => ParseReferenceState::InArraySlice,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InArraySlice => match c {
                '0'..='9' => ParseReferenceState::InArraySlice,
                ']' => ParseReferenceState::CloseArrayIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },

            ParseReferenceState::StartPropertySelection => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InPropertySelection,
                ',' => ParseReferenceState::InPropertyComma,
                '}' => ParseReferenceState::ClosePropertySelection,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InPropertySelection => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InPropertySelection,
                ',' => ParseReferenceState::InPropertyComma,
                '}' => ParseReferenceState::ClosePropertySelection,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InPropertyComma => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InPropertySelection,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::CloseArrayIndex => match c {
                '.' => ParseReferenceState::StartProperty,
                '[' => ParseReferenceState::StartArrayIndex,
                '{' => ParseReferenceState::StartPropertySelection,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::ClosePropertySelection => match c {
                '.' => ParseReferenceState::StartProperty,
                _ => ParseReferenceState::InvalidCharacter,
            },

            _ => todo!(),
        };

        match next_state {
            ParseReferenceState::InvalidCharacter => None,
            o => Some(o),
        }
    }

    fn invalid_access() -> JqlError {
        JqlError::from_kind(JqlErrorKind::InvalidAccess)
    }

    fn parse_usize(raw: &str) -> Result<usize, JqlError> {
        raw.parse::<usize>().map_err(|_| Self::invalid_access())
    }
}

impl FromStr for Reference {
    type Err = JqlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with('.') {
            return Err(Self::invalid_access());
        }

        // Root reference: identity over the current document.
        if s == "." {
            return Ok(Self {
                access: vec![Access::PropertyAccess {
                    property: String::new(),
                    properties: PropertySelection::All,
                }],
            });
        }

        let mut access: Vec<Access> = vec![Access::PropertyAccess {
            property: String::new(),
            properties: PropertySelection::All,
        }];
        let mut idx: usize = 1;
        let mut state = ParseReferenceState::StartProperty;
        let mut prop = access[0].clone();
        let mut start: usize = 1;

        while idx < s.len() {
            let ch = s.chars().nth(idx).ok_or_else(Self::invalid_access)?;
            let Some(new_state) = Self::transition_table(&state, ch) else {
                return Err(Self::invalid_access());
            };

            state = new_state;
            if let Ok(v) = Self::accept_state(&state, start, idx + ch.len_utf8(), s, &prop) {
                prop = v;
            }

            if matches!(
                state,
                ParseReferenceState::StartProperty | ParseReferenceState::StartArrayIndex
            ) {
                access.push(prop);
                start = idx + ch.len_utf8();

                prop = match state {
                    ParseReferenceState::StartProperty => Access::PropertyAccess {
                        property: String::new(),
                        properties: PropertySelection::All,
                    },
                    ParseReferenceState::StartArrayIndex => Access::IndexAccess {
                        index: ArrayIndex::All,
                        properties: PropertySelection::All,
                    },
                    _ => unreachable!(),
                }
            }

            idx += ch.len_utf8();
        }

        access.push(prop);

        match (access.get(0), access.get(1)) {
            (
                Some(Access::PropertyAccess {
                    property: p1,
                    properties: _,
                }),
                Some(Access::PropertyAccess {
                    property: p2,
                    properties: _,
                }),
            ) => {
                if p1 == p2 && p1.is_empty() {
                    access.remove(0);
                }
            }
            _ => unreachable!(),
        }

        match state {
            ParseReferenceState::InProperty
            | ParseReferenceState::CloseArrayIndex
            | ParseReferenceState::ClosePropertySelection => Ok(Self { access }),
            _ => Err(Self::invalid_access()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Access, ArrayIndex, PropertySelection, Reference};
    use crate::jql::error::JqlErrorKind;
    use std::str::FromStr;

    fn parse(source: &str) -> Reference {
        Reference::from_str(source).unwrap_or_else(|err| panic!("should parse {source:?}: {err}"))
    }

    fn parse_err(source: &str) {
        let err = Reference::from_str(source).expect_err(&format!("should reject {source:?}"));
        assert_eq!(err.kind, JqlErrorKind::InvalidAccess, "source {source:?}");
    }

    fn root() -> Access {
        Access::PropertyAccess {
            property: String::new(),
            properties: PropertySelection::All,
        }
    }

    fn prop(name: &str) -> Access {
        Access::PropertyAccess {
            property: name.to_string(),
            properties: PropertySelection::All,
        }
    }

    fn prop_select(name: &str, keys: &[&str]) -> Access {
        Access::PropertyAccess {
            property: name.to_string(),
            properties: PropertySelection::Properties(
                keys.iter().map(|k| (*k).to_string()).collect(),
            ),
        }
    }

    fn index(index: ArrayIndex) -> Access {
        Access::IndexAccess {
            index,
            properties: PropertySelection::All,
        }
    }

    fn index_select(index: ArrayIndex, keys: &[&str]) -> Access {
        Access::IndexAccess {
            index,
            properties: PropertySelection::Properties(
                keys.iter().map(|k| (*k).to_string()).collect(),
            ),
        }
    }

    #[test]
    fn property_only() {
        // .jobs -> List<Job>
        assert_eq!(
            parse(".jobs"),
            Reference {
                access: vec![root(), prop("jobs")],
            }
        );
    }

    #[test]
    fn root_ref() {
        // . -> identity over the document
        assert_eq!(
            parse("."),
            Reference {
                access: vec![root()],
            }
        );
    }

    #[test]
    fn root_with_extra_ref() {
        assert_eq!(
            parse(".{name,age}"),
            Reference {
                access: vec![Access::PropertyAccess {
                    property: String::new(),
                    properties: PropertySelection::Properties(vec![
                        String::from("name"),
                        String::from("age"),
                    ]),
                }],
            }
        );
    }

    #[test]
    fn array_index() {
        // .jobs[1] -> Job
        assert_eq!(
            parse(".jobs[1]"),
            Reference {
                access: vec![root(), prop("jobs"), index(ArrayIndex::Index(1))],
            }
        );
    }

    #[test]
    fn array_index_zero() {
        assert_eq!(
            parse(".jobs[0]"),
            Reference {
                access: vec![root(), prop("jobs"), index(ArrayIndex::Index(0))],
            }
        );
        assert_eq!(
            parse(".jobs[10]"),
            Reference {
                access: vec![root(), prop("jobs"), index(ArrayIndex::Index(10))],
            }
        );
    }

    #[test]
    fn array_all() {
        // .jobs[] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]"),
            Reference {
                access: vec![root(), prop("jobs"), index(ArrayIndex::All)],
            }
        );
    }

    #[test]
    fn array_multiple_index() {
        // .jobs[1,2] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[1,2]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::MultipleIndex(vec![1, 2])),
                ],
            }
        );
    }

    #[test]
    fn array_multiple_index_with_zero() {
        assert_eq!(
            parse(".jobs[0,2]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::MultipleIndex(vec![0, 2])),
                ],
            }
        );
        assert_eq!(
            parse(".jobs[0,1,2]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::MultipleIndex(vec![0, 1, 2])),
                ],
            }
        );
    }

    #[test]
    fn array_range() {
        // .jobs[1:3] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[1:3]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::Range { start: 1, end: 3 }),
                ],
            }
        );
    }

    #[test]
    fn array_range_with_zero() {
        assert_eq!(
            parse(".jobs[0:1]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::Range { start: 0, end: 1 }),
                ],
            }
        );
        assert_eq!(
            parse(".jobs[0:0]"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index(ArrayIndex::Range { start: 0, end: 0 }),
                ],
            }
        );
    }

    #[test]
    fn chained_property_with_zero_index() {
        assert_eq!(
            parse(".company.teams[0].name"),
            Reference {
                access: vec![
                    root(),
                    prop("company"),
                    prop("teams"),
                    index(ArrayIndex::Index(0)),
                    prop("name"),
                ],
            }
        );
    }

    #[test]
    fn rejects_invalid_array_syntax() {
        for source in [
            ".jobs[",
            ".jobs[1",
            ".jobs[1,",
            ".jobs[1:",
            ".jobs[1:2:3]",
            ".jobs[a]",
            ".jobs[-1]",
            ".jobs[1,,2]",
            ".jobs[1::2]",
            "jobs",
            ".jobs]",
        ] {
            parse_err(source);
        }
    }

    #[test]
    fn rejects_invalid_character_instead_of_skipping() {
        // Previously invalid chars were skipped; ensure they hard-fail now.
        parse_err(".jobs[x]");
        parse_err(".count[0a]");
        parse_err(".jobs[1;2]");
    }

    #[test]
    fn object_all() {
        // .person{} -> Iterator<Pair>
        assert_eq!(
            parse(".person{}"),
            Reference {
                access: vec![root(), prop("person")],
            }
        );
    }

    #[test]
    fn object_multiple_properties() {
        // .person{name,age} -> Iterator<Pair if Pair[0] in [name, age]>
        assert_eq!(
            parse(".person{name,age}"),
            Reference {
                access: vec![root(), prop_select("person", &["name", "age"])],
            }
        );
    }

    #[test]
    fn array_all_then_object_all() {
        // .jobs[]{} -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]{}"),
            Reference {
                access: vec![root(), prop("jobs"), index(ArrayIndex::All)],
            }
        );
    }

    #[test]
    fn array_all_then_object_multiple_properties() {
        // .jobs[]{title,description} -> Iterator<Pair if Pair[0] in [title, description]>
        assert_eq!(
            parse(".jobs[]{title,description}"),
            Reference {
                access: vec![
                    root(),
                    prop("jobs"),
                    index_select(ArrayIndex::All, &["title", "description"]),
                ],
            }
        );
    }
}
