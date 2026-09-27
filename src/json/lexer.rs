use crate::json::{
    error::JsonParserError,
    escape::{ESCAPE_TOKENS, MUST_BE_ESCAPED},
    token::{JsonToken, JsonTokenKind, Span},
};
use crate::source::Source;

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    index: usize,
    source: Source<'a>,
}

// support -1.2e-3
#[derive(Debug, Clone, Eq, PartialEq)]
enum ParseNumberState {
    Start,
    Signed,
    Integer,
    StartDecimal,
    InDecimal,
    StartExp, // at E or e
    ExpSign,
    ExpValue,
    OnError,
    OnSeparator,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: Source::new(source),
            index: 0,
        }
    }

    pub fn next_token(&mut self) -> Result<JsonToken, JsonParserError> {
        if self.index >= self.source.len() {
            return Ok(JsonToken {
                kind: JsonTokenKind::Stop,
                span: Span {
                    start: self.source.len(),
                    end: self.source.len() + 1,
                },
            });
        }

        let c = self.source.char_at(self.index).unwrap();
        let pos = self.index;
        let (token, next_index) = match c {
            '{' => (
                JsonToken {
                    kind: JsonTokenKind::LBrace,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            '}' => (
                JsonToken {
                    kind: JsonTokenKind::RBrace,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            '[' => (
                JsonToken {
                    kind: JsonTokenKind::LBracket,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            ']' => (
                JsonToken {
                    kind: JsonTokenKind::RBracket,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            ':' => (
                JsonToken {
                    kind: JsonTokenKind::Colon,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            ',' => (
                JsonToken {
                    kind: JsonTokenKind::Comma,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            ' ' | '\n' | '\r' | '\t' => (
                JsonToken {
                    kind: JsonTokenKind::Whitespace,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                pos + 1,
            ),
            '"' => self.parse_string()?,
            't' => self.parse_literal(JsonTokenKind::True)?,
            'f' => self.parse_literal(JsonTokenKind::False)?,
            'n' => self.parse_literal(JsonTokenKind::Null)?,
            '0'..='9' | '-' => self.parse_number()?,
            _ => (
                JsonToken {
                    kind: JsonTokenKind::InvalidToken,
                    span: Span {
                        start: pos,
                        end: pos + 1,
                    },
                },
                0,
            ),
        };

        if token.kind == JsonTokenKind::InvalidToken {
            return Err(JsonParserError::LexicalError(String::from(
                "invalid character",
            )));
        }

        self.index = next_index;
        Ok(token)
    }

    fn parse_literal(
        &self,
        token_type: JsonTokenKind,
    ) -> Result<(JsonToken, usize), JsonParserError> {
        let s = match token_type {
            JsonTokenKind::True => "true",
            JsonTokenKind::False => "false",
            JsonTokenKind::Null => "null",
            _ => "",
        };
        let err = Err(JsonParserError::LexicalError(String::from(
            "invalid literal",
        )));
        if s.is_empty() {
            return err;
        }

        let span = Span {
            start: self.index,
            end: self.index + s.len(),
        };

        if self.get_str(&span) == Some(s) {
            return Ok((
                JsonToken {
                    kind: token_type,
                    span,
                },
                span.end,
            ));
        }

        err
    }

    fn has_leading_zero(value: &str) -> bool {
        let int_part = value.strip_prefix('-').unwrap_or(value);
        matches!(int_part.as_bytes(), [b'0', b'0'..=b'9', ..])
    }

    fn parse_number(&self) -> Result<(JsonToken, usize), JsonParserError> {
        /*
         * Follow this Finite state machine to implement JSON number validation
         *
         * state = 0: in start
         * state = 1: in signed
         * state = 2: in interger
         * state = 3: at dot
         * state = 4: after dot
         * state = 5: at e
         * state = 6: dec sign
         * state = 7: dec value
         */

        if Self::has_leading_zero(self.source.data) {
            return Err(JsonParserError::LexicalError(String::from(
                "invalid number",
            )));
        }

        let mut span = Span {
            start: self.index,
            end: self.index,
        };
        let value = self
            .source
            .slice_at(self.index)
            .ok_or(JsonParserError::ParserError(String::from("invalid number")))?;

        let mut state: ParseNumberState = ParseNumberState::Start;
        for c in value.chars() {
            let next_state = match c {
                '-' => match state {
                    ParseNumberState::Start => ParseNumberState::Signed,
                    ParseNumberState::StartExp => ParseNumberState::ExpSign,
                    _ => ParseNumberState::OnError,
                },
                '0'..='9' => match state {
                    ParseNumberState::Start
                    | ParseNumberState::Signed
                    | ParseNumberState::Integer => ParseNumberState::Integer,
                    ParseNumberState::StartDecimal | ParseNumberState::InDecimal => {
                        ParseNumberState::InDecimal
                    }
                    ParseNumberState::StartExp
                    | ParseNumberState::ExpSign
                    | ParseNumberState::ExpValue => ParseNumberState::ExpValue,
                    _ => ParseNumberState::OnError,
                },
                '.' => match state {
                    ParseNumberState::Integer => ParseNumberState::StartDecimal,
                    _ => ParseNumberState::OnError,
                },
                'e' | 'E' => match state {
                    ParseNumberState::InDecimal | ParseNumberState::Integer => {
                        ParseNumberState::StartExp
                    }
                    _ => ParseNumberState::OnError,
                },
                '+' => match state {
                    ParseNumberState::StartExp => ParseNumberState::ExpSign,
                    _ => ParseNumberState::OnError,
                },
                '{' | '}' | '[' | ']' | ',' | ' ' | ':' | '\r' | '\t' | '\n' => {
                    ParseNumberState::OnSeparator
                }
                _ => ParseNumberState::OnError,
            };

            match next_state {
                ParseNumberState::OnError | ParseNumberState::OnSeparator => break,
                _ => {
                    state = next_state;
                    span.end += 1;
                }
            }
        }

        match state {
            ParseNumberState::OnSeparator
            | ParseNumberState::InDecimal
            | ParseNumberState::Integer
            | ParseNumberState::ExpValue => Ok((
                JsonToken {
                    kind: JsonTokenKind::Number,
                    span,
                },
                span.end,
            )),
            _ => Err(JsonParserError::LexicalError(String::from(
                "invalid number",
            ))),
        }
    }

    fn must_be_escaped(ch: char) -> bool {
        let mut buf = [0u8; 4];
        let encoded: &str = ch.encode_utf8(&mut buf);
        MUST_BE_ESCAPED.contains(&encoded)
    }

    fn parse_string(&mut self) -> Result<(JsonToken, usize), JsonParserError> {
        let source = self.source;
        let mut idx = self.index + 1;

        while idx < source.len() {
            let c = source.char_at(idx);
            if c == Some('\\') {
                let escape = source
                    .slice(idx, idx + 2)
                    .filter(|s| ESCAPE_TOKENS.contains(s))
                    .or_else(|| {
                        source
                            .slice(idx, idx + 6)
                            .filter(|s| Self::is_hex_escape(s))
                    });
                let Some(s) = escape else { break };
                idx += s.len();
                continue;
            } else if c == Some('"') {
                return Ok((
                    JsonToken {
                        kind: JsonTokenKind::String,
                        span: Span {
                            start: self.index,
                            end: idx + 1,
                        },
                    },
                    idx + 1,
                ));
            } else if c.is_some_and(Self::must_be_escaped) {
                break;
            } else {
                idx += 1;
            }
        }

        Err(JsonParserError::LexicalError(String::from(
            "invalid string",
        )))
    }

    fn is_hex_escape(value: &str) -> bool {
        value.len() == 6
            && value.starts_with("\\u")
            && value
                .get(2..)
                .unwrap()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }

    pub fn tokenize(source: &'a str) -> Result<Vec<JsonToken>, JsonParserError> {
        let mut lexer = Self::new(source);
        let mut tokens: Vec<JsonToken> = vec![];

        loop {
            let token = lexer.next_token()?;
            match token.kind {
                JsonTokenKind::Stop => break,
                JsonTokenKind::Whitespace => continue,
                _ => tokens.push(token),
            }
        }

        Ok(tokens)
    }

    fn get_str(&self, span: &Span) -> Option<&str> {
        self.source.slice(span.start, span.end)
    }
}

#[cfg(test)]
mod test {
    use crate::json::{
        error::JsonParserError,
        lexer::{JsonToken, JsonTokenKind, Lexer, Span},
    };

    fn run(source: &str) -> Result<Vec<JsonToken>, JsonParserError> {
        let tokens = Lexer::tokenize(source)?;
        for token in tokens.clone() {
            println!("{}", token.display(source));
        }

        Ok(tokens)
    }

    fn tok(kind: JsonTokenKind, start: usize, end: usize) -> JsonToken {
        JsonToken {
            kind,
            span: Span { start, end },
        }
    }

    fn kinds(tokens: &[JsonToken]) -> Vec<JsonTokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn structural_characters() {
        let input: [&str; 6] = ["{", "}", "[", "]", ":", ","];
        let expected: [JsonTokenKind; 6] = [
            JsonTokenKind::LBrace,
            JsonTokenKind::RBrace,
            JsonTokenKind::LBracket,
            JsonTokenKind::RBracket,
            JsonTokenKind::Colon,
            JsonTokenKind::Comma,
        ];
        for (s, kind) in input.iter().zip(expected.iter()) {
            assert_eq!(
                run(s),
                Ok(vec![tok(kind.clone(), 0, s.len())]),
                "structural {:?}",
                s
            );
        }
    }

    #[test]
    fn whitespace_is_insignificant() {
        let input: [&str; 5] = ["", " ", "\t", "\n", "\r"];
        for s in input {
            assert_eq!(run(s), Ok(vec![]), "whitespace {:?}", s);
        }

        let padded: [&str; 4] = ["  true  ", "\tfalse\t", "\nnull\n", "\r1\r"];
        let expected_kinds: [JsonTokenKind; 4] = [
            JsonTokenKind::True,
            JsonTokenKind::False,
            JsonTokenKind::Null,
            JsonTokenKind::Number,
        ];
        for (s, kind) in padded.iter().zip(expected_kinds.iter()) {
            let tokens = run(s).expect("should lex");
            assert_eq!(kinds(&tokens), vec![kind.clone()], "padded {:?}", s);
        }
    }

    #[test]
    fn literals() {
        let input: [&str; 3] = ["true", "false", "null"];
        let expected: [JsonToken; 3] = [
            tok(JsonTokenKind::True, 0, 4),
            tok(JsonTokenKind::False, 0, 5),
            tok(JsonTokenKind::Null, 0, 4),
        ];
        for (s, expected) in input.iter().zip(expected.iter()) {
            assert_eq!(&run(s), &Ok(vec![expected.clone()]));
        }
    }

    #[test]
    fn incomplete_or_invalid_literals_are_errors() {
        let input: [&str; 9] = [
            "tru", "tr", "t", "fals", "fal", "nul", "nu", "TRUE", "False",
        ];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn strings_valid() {
        let cases: [(&str, usize, usize); 6] = [
            ("\"\"", 0, 2),
            ("\"hello\"", 0, 7),
            ("\" \"", 0, 3),
            ("\"\\\"\"", 0, 4),
            ("\"\\\\\"", 0, 4),
            ("\"\\n\\t\\r\\b\\f\\/\"", 0, 14),
        ];
        for (s, start, end) in cases {
            assert_eq!(
                run(s),
                Ok(vec![tok(JsonTokenKind::String, start, end)]),
                "string {:?}",
                s
            );
        }
    }

    #[test]
    fn strings_unicode_escape() {
        let input = "\"\\u0041\"";
        assert_eq!(
            run(input),
            Ok(vec![tok(JsonTokenKind::String, 0, input.len())])
        );
        let input = "\"\\uD83D\\uDE00\"";
        assert_eq!(
            run(input),
            Ok(vec![tok(JsonTokenKind::String, 0, input.len())])
        );
    }

    #[test]
    fn strings_raw_unicode() {
        let cases: [&str; 5] = ["\"é\"", "\"€\"", "\"🙂\"", "\"café\"", "\"é\\n🙂\""];
        for s in cases {
            assert_eq!(
                run(s),
                Ok(vec![tok(JsonTokenKind::String, 0, s.len())]),
                "string {:?}",
                s
            );
        }
    }

    #[test]
    fn unicode_string_keeps_following_byte_spans() {
        let input = "\"é\",1";
        assert_eq!(
            run(input),
            Ok(vec![
                tok(JsonTokenKind::String, 0, 4),
                tok(JsonTokenKind::Comma, 4, 5),
                tok(JsonTokenKind::Number, 5, 6),
            ])
        );

        let input = "{\"a\":\"é\"}";
        assert_eq!(
            run(input),
            Ok(vec![
                tok(JsonTokenKind::LBrace, 0, 1),
                tok(JsonTokenKind::String, 1, 4),
                tok(JsonTokenKind::Colon, 4, 5),
                tok(JsonTokenKind::String, 5, 9),
                tok(JsonTokenKind::RBrace, 9, 10),
            ])
        );

        let input = "[\"🙂\"]";
        assert_eq!(
            run(input),
            Ok(vec![
                tok(JsonTokenKind::LBracket, 0, 1),
                tok(JsonTokenKind::String, 1, 7),
                tok(JsonTokenKind::RBracket, 7, 8),
            ])
        );
    }

    #[test]
    fn raw_unicode_outside_string_is_error() {
        let input: [&str; 4] = ["é", "€", "🙂", "\u{00A0}1"];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn unescaped_controls_are_errors() {
        for ch in '\u{0000}'..='\u{001F}' {
            let input = format!("\"{ch}\"");
            assert!(
                run(&input).is_err(),
                "expected lexical error for U+{:04X}",
                u32::from(ch)
            );
        }

        let embedded: [&str; 3] = ["\"a\rb\"", "\"é\u{0001}\"", "\"\u{001F}x\""];
        for s in embedded {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn escaped_controls_are_strings() {
        let cases: [&str; 6] = [
            "\"\\u0000\"",
            "\"\\u0001\"",
            "\"\\u001F\"",
            "\"\\n\"",
            "\"\\r\"",
            "\"\\t\"",
        ];
        for s in cases {
            assert_eq!(
                run(s),
                Ok(vec![tok(JsonTokenKind::String, 0, s.len())]),
                "escaped {:?}",
                s
            );
        }
    }

    #[test]
    fn strings_invalid_are_errors() {
        let input: [&str; 8] = [
            "\"",
            "\"abc",
            "\"\\x\"",
            "\"\\u\"",
            "\"\\u12\"",
            "\"\\uZZZZ\"",
            "\"\n\"",
            "\"\t\"",
        ];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn numbers_valid() {
        let input: [&str; 15] = [
            "0", "1", "10", "100", "123", "-0", "-1", "-10", "0.1", "3.14", "10.0", "1e2", "1E2",
            "1e+2", "1e-2",
        ];
        for s in input {
            assert_eq!(
                run(s),
                Ok(vec![tok(JsonTokenKind::Number, 0, s.len())]),
                "number {:?}",
                s
            );
        }
    }

    #[test]
    fn numbers_with_fraction_and_exponent() {
        let input: [&str; 4] = ["1.2e3", "1.2E+3", "-1.2e-3", "0.0e0"];
        for s in input {
            assert_eq!(
                run(s),
                Ok(vec![tok(JsonTokenKind::Number, 0, s.len())]),
                "number {:?}",
                s
            );
        }
    }

    #[test]
    fn numbers_invalid_are_errors() {
        let input: [&str; 14] = [
            "+", "+1", "01", "-01", "1.", ".1", "-.1", "1e", "1e+", "1e-", "--1", "1.2.3", "1ee2",
            "0x1",
        ];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn number_then_structural() {
        let cases: [(&str, &[JsonTokenKind]); 3] = [
            ("10,", &[JsonTokenKind::Number, JsonTokenKind::Comma]),
            ("10]", &[JsonTokenKind::Number, JsonTokenKind::RBracket]),
            ("10}", &[JsonTokenKind::Number, JsonTokenKind::RBrace]),
        ];
        for (s, expected) in cases {
            let tokens = run(s).expect("should lex");
            assert_eq!(kinds(&tokens), expected, "source {:?}", s);
            assert_eq!(tokens[0].span, Span { start: 0, end: 2 });
        }
    }

    #[test]
    fn array_tokens() {
        let input = "[1,2,true,false,null,\"x\"]";
        let tokens = run(input).expect("should lex");
        assert_eq!(
            kinds(&tokens),
            vec![
                JsonTokenKind::LBracket,
                JsonTokenKind::Number,
                JsonTokenKind::Comma,
                JsonTokenKind::Number,
                JsonTokenKind::Comma,
                JsonTokenKind::True,
                JsonTokenKind::Comma,
                JsonTokenKind::False,
                JsonTokenKind::Comma,
                JsonTokenKind::Null,
                JsonTokenKind::Comma,
                JsonTokenKind::String,
                JsonTokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn object_tokens() {
        let input = "{\"a\":1,\"b\":true}";
        let tokens = run(input).expect("should lex");
        assert_eq!(
            kinds(&tokens),
            vec![
                JsonTokenKind::LBrace,
                JsonTokenKind::String,
                JsonTokenKind::Colon,
                JsonTokenKind::Number,
                JsonTokenKind::Comma,
                JsonTokenKind::String,
                JsonTokenKind::Colon,
                JsonTokenKind::True,
                JsonTokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn nested_structure_tokens() {
        let input = "{\"arr\":[1,{\"k\":null}]}";
        let tokens = run(input).expect("should lex");
        assert_eq!(
            kinds(&tokens),
            vec![
                JsonTokenKind::LBrace,
                JsonTokenKind::String,
                JsonTokenKind::Colon,
                JsonTokenKind::LBracket,
                JsonTokenKind::Number,
                JsonTokenKind::Comma,
                JsonTokenKind::LBrace,
                JsonTokenKind::String,
                JsonTokenKind::Colon,
                JsonTokenKind::Null,
                JsonTokenKind::RBrace,
                JsonTokenKind::RBracket,
                JsonTokenKind::RBrace,
            ]
        );
    }

    #[test]
    fn unknown_characters_are_errors() {
        let input: [&str; 6] = ["@", "#", "'abc'", "NaN", "Infinity", "-Infinity"];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }

    #[test]
    fn literals_must_be_complete_tokens() {
        let input: [&str; 3] = ["truex", "falsey", "nullable"];
        for s in input {
            assert!(run(s).is_err(), "expected lexical error for {:?}", s);
        }
    }
}
