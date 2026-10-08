use std::{fmt::Debug, str::FromStr};

use crate::jql::error::{JqlError, JqlErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayExtra {
    Range { start: usize, end: usize },
    Index(usize),
    MultipleIndex(Vec<usize>),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectExtra {
    MultipleProperties(Vec<String>),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub array_extra: ArrayExtra,
    pub object_extra: ObjectExtra,
}

impl std::default::Default for Property {
    fn default() -> Self {
        Self {
            name: String::new(),
            array_extra: ArrayExtra::All,
            object_extra: ObjectExtra::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone)]
enum ParseReferenceState {
    StartProperty,
    InProperty,
    StartExtraArray,
    InIndex,
    InSlice,
    InSliceColon,
    InMultipleIndex,
    InMultipleIndexComma,
    CloseExtraArray,
    StartExtraObject,
    InExtraObjectKey,
    InExtraObjectComma,
    CloseExtraObject,
    InvalidCharacter,
}

impl Reference {
    fn invalid_access() -> JqlError {
        JqlError::from_kind(JqlErrorKind::InvalidAccess)
    }

    fn parse_usize(raw: &str) -> Result<usize, JqlError> {
        raw.parse::<usize>().map_err(|_| Self::invalid_access())
    }

    fn accept_state(
        state: &ParseReferenceState,
        start: usize,
        end: usize,
        source: &str,
        prop: &mut Property,
    ) -> Result<(), JqlError> {
        match state {
            ParseReferenceState::InProperty => {
                prop.name = source
                    .get(start..end)
                    .ok_or_else(Self::invalid_access)?
                    .to_string();
                Ok(())
            }
            ParseReferenceState::CloseExtraObject => {
                let s = source.get(start..end).ok_or_else(Self::invalid_access)?;
                if let Some(lbrace) = s.find('{') {
                    let keys: Vec<String> = s
                        .get(lbrace + 1..end - 2)
                        .ok_or_else(Self::invalid_access)?
                        .split(",")
                        .map(|x| x.to_string())
                        .filter(|x| !x.is_empty())
                        .collect();
                    if keys.is_empty() {
                        prop.object_extra = ObjectExtra::All;
                    } else {
                        prop.object_extra = ObjectExtra::MultipleProperties(keys);
                    }
                }
                Ok(())
            }
            ParseReferenceState::CloseExtraArray => Ok(()),
            ParseReferenceState::InIndex => {
                let s = source.get(start..end).ok_or_else(Self::invalid_access)?;
                if let Some(x) = s.find('[') {
                    let index = Self::parse_usize(&s[x + 1..])?;
                    prop.array_extra = ArrayExtra::Index(index);
                }
                Ok(())
            }
            ParseReferenceState::InSlice => {
                let s = source.get(start..end).ok_or_else(Self::invalid_access)?;
                if let Some(index) = s.find('[') {
                    let parts: Vec<&str> = s[index + 1..].split(':').collect();
                    if parts.len() != 2 {
                        return Err(Self::invalid_access());
                    }
                    prop.array_extra = ArrayExtra::Range {
                        start: Self::parse_usize(parts[0])?,
                        end: Self::parse_usize(parts[1])?,
                    };
                }
                Ok(())
            }
            ParseReferenceState::InMultipleIndex => {
                let s = source.get(start..end).ok_or_else(Self::invalid_access)?;
                if let Some(index) = s.find('[') {
                    let indices = s[index + 1..]
                        .split(',')
                        .map(Self::parse_usize)
                        .collect::<Result<Vec<_>, _>>()?;
                    prop.array_extra = ArrayExtra::MultipleIndex(indices);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn transition_table(state: &ParseReferenceState, c: char) -> Option<ParseReferenceState> {
        let next_state = match state {
            ParseReferenceState::StartProperty => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InProperty,
                '{' => ParseReferenceState::StartExtraObject,
                '[' => ParseReferenceState::StartExtraArray,
                '.' => ParseReferenceState::StartProperty,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InProperty => match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => ParseReferenceState::InProperty,
                '.' => ParseReferenceState::StartProperty,
                '[' => ParseReferenceState::StartExtraArray,
                '{' => ParseReferenceState::StartExtraObject,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::StartExtraArray => match c {
                '0'..='9' => ParseReferenceState::InIndex,
                ']' => ParseReferenceState::CloseExtraArray,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InIndex => match c {
                '0'..='9' => ParseReferenceState::InIndex,
                ']' => ParseReferenceState::CloseExtraArray,
                ',' => ParseReferenceState::InMultipleIndexComma,
                ':' => ParseReferenceState::InSliceColon,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InMultipleIndexComma => match c {
                '0'..='9' => ParseReferenceState::InMultipleIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InMultipleIndex => match c {
                '0'..='9' => ParseReferenceState::InMultipleIndex,
                ',' => ParseReferenceState::InMultipleIndexComma,
                ']' => ParseReferenceState::CloseExtraArray,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InSliceColon => match c {
                '0'..='9' => ParseReferenceState::InSlice,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InSlice => match c {
                '0'..='9' => ParseReferenceState::InSlice,
                ']' => ParseReferenceState::CloseExtraArray,
                _ => ParseReferenceState::InvalidCharacter,
            },

            ParseReferenceState::StartExtraObject => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InExtraObjectKey,
                ',' => ParseReferenceState::InExtraObjectComma,
                '}' => ParseReferenceState::CloseExtraObject,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InExtraObjectKey => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InExtraObjectKey,
                ',' => ParseReferenceState::InExtraObjectComma,
                '}' => ParseReferenceState::CloseExtraObject,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InExtraObjectComma => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InExtraObjectKey,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::CloseExtraArray => match c {
                '.' => ParseReferenceState::StartProperty,
                '{' => ParseReferenceState::StartExtraObject,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::CloseExtraObject => match c {
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
}

impl FromStr for Reference {
    type Err = JqlError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /*
         * Support
         * ---------------------------
         * .          -> root (identity); always prefixed as Property::default()
         * .jobs      -> List<Job>
         * .jobs[1]   -> Job
         * .jobs[]    -> Iterator<Job>
         * .jobs[1,2] -> Iterator<Job>
         * .jobs[1:3] -> Iterator<Job>
         *
         * .person{}           -> Iterator<Pair>
         * .person{name,age}   -> Iterator<Pair if Pair[0] in [name, age]>
         *
         * .jobs[]{}  -> Iterator<Job>
         * .jobs[]{title,description} -> Iterator<Pair if Pair[0] in [title, description]>
         *
         */

        if !s.starts_with('.') {
            return Err(Self::invalid_access());
        }

        // Root reference: identity over the current document.
        if s == "." {
            return Ok(Self {
                properties: vec![Property::default()],
            });
        }

        let mut props: Vec<Property> = vec![Property::default()];
        let mut idx: usize = 1;
        let mut state = ParseReferenceState::StartProperty;
        let mut prop = Property::default();
        let mut start: usize = 1;

        while idx < s.len() {
            let ch = s.chars().nth(idx).ok_or_else(Self::invalid_access)?;
            let Some(new_state) = Self::transition_table(&state, ch) else {
                return Err(Self::invalid_access());
            };

            state = new_state;
            Self::accept_state(&state, start, idx + ch.len_utf8(), s, &mut prop)?;

            if matches!(state, ParseReferenceState::StartProperty) {
                props.push(prop);
                start = idx + ch.len_utf8();
                prop = Property::default();
            }

            idx += ch.len_utf8();
        }

        props.push(prop);

        if props.len() > 1 && props[0].name == props[1].name && props[0].name == String::new() {
            props.remove(0);
        }

        match state {
            ParseReferenceState::InProperty
            | ParseReferenceState::CloseExtraArray
            | ParseReferenceState::CloseExtraObject => Ok(Self { properties: props }),
            _ => Err(Self::invalid_access()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ArrayExtra, ObjectExtra, Property, Reference};
    use crate::jql::error::JqlErrorKind;
    use std::str::FromStr;

    fn parse(source: &str) -> Reference {
        Reference::from_str(source).unwrap_or_else(|err| panic!("should parse {source:?}: {err}"))
    }

    fn parse_err(source: &str) {
        let err = Reference::from_str(source).expect_err(&format!("should reject {source:?}"));
        assert_eq!(err.kind, JqlErrorKind::InvalidAccess, "source {source:?}");
    }

    fn prop(name: &str, array_extra: ArrayExtra, object_extra: ObjectExtra) -> Property {
        Property {
            name: name.to_string(),
            array_extra,
            object_extra,
        }
    }

    fn root() -> Property {
        Property::default()
    }

    #[test]
    fn root_ref() {
        // . -> identity over the document
        assert_eq!(
            parse("."),
            Reference {
                properties: vec![root()],
            }
        );
    }

    #[test]
    fn root_with_extra_ref() {
        // . -> identity over the document
        assert_eq!(
            parse(".{name,age}"),
            Reference {
                properties: vec![Property {
                    name: String::new(),
                    array_extra: ArrayExtra::All,
                    object_extra: ObjectExtra::MultipleProperties(vec![
                        String::from("name"),
                        String::from("age")
                    ])
                }],
            }
        );
    }

    #[test]
    fn property_only() {
        // .jobs -> List<Job>
        assert_eq!(
            parse(".jobs"),
            Reference {
                properties: vec![root(), prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_index() {
        // .jobs[1] -> Job
        assert_eq!(
            parse(".jobs[1]"),
            Reference {
                properties: vec![root(), prop("jobs", ArrayExtra::Index(1), ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_index_zero() {
        assert_eq!(
            parse(".jobs[0]"),
            Reference {
                properties: vec![root(), prop("jobs", ArrayExtra::Index(0), ObjectExtra::All)],
            }
        );
        assert_eq!(
            parse(".jobs[10]"),
            Reference {
                properties: vec![
                    root(),
                    prop("jobs", ArrayExtra::Index(10), ObjectExtra::All)
                ],
            }
        );
    }

    #[test]
    fn array_all() {
        // .jobs[] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]"),
            Reference {
                properties: vec![root(), prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_multiple_index() {
        // .jobs[1,2] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[1,2]"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::MultipleIndex(vec![1, 2]),
                        ObjectExtra::All,
                    )
                ],
            }
        );
    }

    #[test]
    fn array_multiple_index_with_zero() {
        assert_eq!(
            parse(".jobs[0,2]"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::MultipleIndex(vec![0, 2]),
                        ObjectExtra::All,
                    )
                ],
            }
        );
        assert_eq!(
            parse(".jobs[0,1,2]"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::MultipleIndex(vec![0, 1, 2]),
                        ObjectExtra::All,
                    )
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
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::Range { start: 1, end: 3 },
                        ObjectExtra::All,
                    )
                ],
            }
        );
    }

    #[test]
    fn array_range_with_zero() {
        assert_eq!(
            parse(".jobs[0:1]"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::Range { start: 0, end: 1 },
                        ObjectExtra::All,
                    )
                ],
            }
        );
        assert_eq!(
            parse(".jobs[0:0]"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::Range { start: 0, end: 0 },
                        ObjectExtra::All,
                    )
                ],
            }
        );
    }

    #[test]
    fn chained_property_with_zero_index() {
        assert_eq!(
            parse(".company.teams[0].name"),
            Reference {
                properties: vec![
                    root(),
                    prop("company", ArrayExtra::All, ObjectExtra::All),
                    prop("teams", ArrayExtra::Index(0), ObjectExtra::All),
                    prop("name", ArrayExtra::All, ObjectExtra::All),
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
                properties: vec![root(), prop("person", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn object_multiple_properties() {
        // .person{name,age} -> Iterator<Pair if Pair[0] in [name, age]>
        assert_eq!(
            parse(".person{name,age}"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "person",
                        ArrayExtra::All,
                        ObjectExtra::MultipleProperties(vec![
                            "name".to_string(),
                            "age".to_string(),
                        ]),
                    )
                ],
            }
        );
    }

    #[test]
    fn array_all_then_object_all() {
        // .jobs[]{} -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]{}"),
            Reference {
                properties: vec![root(), prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_all_then_object_multiple_properties() {
        // .jobs[]{title,description} -> Iterator<Pair if Pair[0] in [title, description]>
        assert_eq!(
            parse(".jobs[]{title,description}"),
            Reference {
                properties: vec![
                    root(),
                    prop(
                        "jobs",
                        ArrayExtra::All,
                        ObjectExtra::MultipleProperties(vec![
                            "title".to_string(),
                            "description".to_string(),
                        ]),
                    )
                ],
            }
        );
    }
}
