use std::{fmt::Debug, str::FromStr};

use crate::jql::proxy::ProxyError;

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
    fn accept_state(
        state: &ParseReferenceState,
        start: usize,
        end: usize,
        source: &str,
        prop: &mut Property,
    ) -> bool {
        match state {
            ParseReferenceState::InProperty => {
                prop.name = source.get(start..end).unwrap().to_string();
                true
            }
            ParseReferenceState::CloseExtraObject => {
                let s = source.get(start..end).unwrap();
                if let Some(lbrace) = s.find('{') {
                    let keys: Vec<String> = s
                        .get(lbrace + 1..end - 2)
                        .unwrap()
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

                true
            }
            ParseReferenceState::CloseExtraArray => true,
            ParseReferenceState::InIndex => {
                let s = source.get(start..end).unwrap();
                if let Some(x) = s.find('[') {
                    let index = s[x + 1..].to_string();
                    prop.array_extra = ArrayExtra::Index(index.parse::<usize>().unwrap());
                }
                false
            }
            ParseReferenceState::InSlice => {
                let s = source.get(start..end).unwrap();
                if let Some(index) = s.find('[') {
                    let slices: Vec<usize> = s[index + 1..]
                        .split(':')
                        .map(|x| x.parse::<usize>().unwrap())
                        .collect();
                    prop.array_extra = ArrayExtra::Range {
                        start: slices[0],
                        end: slices[1],
                    }
                }
                false
            }
            ParseReferenceState::InMultipleIndex => {
                let s = source.get(start..end).unwrap();
                if let Some(index) = s.find('[') {
                    let indices: Vec<usize> = s[index + 1..]
                        .split(',')
                        .map(|x| x.parse::<usize>().unwrap())
                        .collect();
                    prop.array_extra = ArrayExtra::MultipleIndex(indices);
                }
                false
            }
            _ => false,
        }
    }

    fn transition_table(state: &ParseReferenceState, c: char) -> Option<ParseReferenceState> {
        let next_state = match state {
            ParseReferenceState::StartProperty => match c {
                'a'..='z' | 'A'..='Z' => ParseReferenceState::InProperty,
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
                '1'..='9' => ParseReferenceState::InIndex,
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
                '1'..='9' => ParseReferenceState::InMultipleIndex,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InMultipleIndex => match c {
                '0'..='9' => ParseReferenceState::InMultipleIndex,
                ',' => ParseReferenceState::InMultipleIndexComma,
                ']' => ParseReferenceState::CloseExtraArray,
                _ => ParseReferenceState::InvalidCharacter,
            },
            ParseReferenceState::InSliceColon => match c {
                '1'..='9' => ParseReferenceState::InSlice,
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
    type Err = ProxyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /*
         * Support
         * ---------------------------
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

        let mut props = Vec::<Property>::new();
        let mut idx: usize = 1;
        let mut state = ParseReferenceState::StartProperty;
        let mut prop = Property::default();
        let mut start: usize = 1;

        while idx < s.len() {
            let ch = s.chars().nth(idx).unwrap();
            if let Some(new_state) = Self::transition_table(&state, ch) {
                state = new_state;
                Self::accept_state(&state, start, idx + ch.len_utf8(), s, &mut prop);

                if matches!(state, ParseReferenceState::StartProperty) {
                    props.push(prop);

                    start = idx + ch.len_utf8();
                    prop = Property::default();
                } else if matches!(state, ParseReferenceState::InvalidCharacter) {
                    return Err(ProxyError::GenericError);
                }
            }

            idx += ch.len_utf8();
        }

        props.push(prop);

        match state {
            ParseReferenceState::InProperty
            | ParseReferenceState::CloseExtraArray
            | ParseReferenceState::CloseExtraObject => Ok(Self { properties: props }),
            _ => Err(ProxyError::GenericError),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ArrayExtra, ObjectExtra, Property, Reference};
    use std::str::FromStr;

    fn parse(source: &str) -> Reference {
        Reference::from_str(source).expect(&format!("should parse {source:?}"))
    }

    fn prop(name: &str, array_extra: ArrayExtra, object_extra: ObjectExtra) -> Property {
        Property {
            name: name.to_string(),
            array_extra,
            object_extra,
        }
    }

    #[test]
    fn property_only() {
        // .jobs -> List<Job>
        assert_eq!(
            parse(".jobs"),
            Reference {
                properties: vec![prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_index() {
        // .jobs[1] -> Job
        assert_eq!(
            parse(".jobs[1]"),
            Reference {
                properties: vec![prop("jobs", ArrayExtra::Index(1), ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_all() {
        // .jobs[] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]"),
            Reference {
                properties: vec![prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_multiple_index() {
        // .jobs[1,2] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[1,2]"),
            Reference {
                properties: vec![prop(
                    "jobs",
                    ArrayExtra::MultipleIndex(vec![1, 2]),
                    ObjectExtra::All,
                )],
            }
        );
    }

    #[test]
    fn array_range() {
        // .jobs[1:3] -> Iterator<Job>
        assert_eq!(
            parse(".jobs[1:3]"),
            Reference {
                properties: vec![prop(
                    "jobs",
                    ArrayExtra::Range { start: 1, end: 3 },
                    ObjectExtra::All,
                )],
            }
        );
    }

    #[test]
    fn object_all() {
        // .person{} -> Iterator<Pair>
        assert_eq!(
            parse(".person{}"),
            Reference {
                properties: vec![prop("person", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn object_multiple_properties() {
        // .person{name,age} -> Iterator<Pair if Pair[0] in [name, age]>
        assert_eq!(
            parse(".person{name,age}"),
            Reference {
                properties: vec![prop(
                    "person",
                    ArrayExtra::All,
                    ObjectExtra::MultipleProperties(vec!["name".to_string(), "age".to_string(),]),
                )],
            }
        );
    }

    #[test]
    fn array_all_then_object_all() {
        // .jobs[]{} -> Iterator<Job>
        assert_eq!(
            parse(".jobs[]{}"),
            Reference {
                properties: vec![prop("jobs", ArrayExtra::All, ObjectExtra::All)],
            }
        );
    }

    #[test]
    fn array_all_then_object_multiple_properties() {
        // .jobs[]{title,description} -> Iterator<Pair if Pair[0] in [title, description]>
        assert_eq!(
            parse(".jobs[]{title,description}"),
            Reference {
                properties: vec![prop(
                    "jobs",
                    ArrayExtra::All,
                    ObjectExtra::MultipleProperties(vec![
                        "title".to_string(),
                        "description".to_string(),
                    ]),
                )],
            }
        );
    }
}
