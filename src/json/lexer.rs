use std::io::Read;

use crate::json::{
    error::{JsonError, JsonErrorKind},
    escape::{ESCAPE_TOKENS, MUST_BE_ESCAPED, UnescapeError, unescape_json_string},
    source::SourceBuffer,
    token::{JsonToken, JsonTokenKind},
};
use crate::{Location, Span};

#[derive(Debug)]
pub struct Lexer<R: Read> {
    source: SourceBuffer<R>,
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

impl<'a> Lexer<&'a [u8]> {
    pub fn from_str(source: &'a str) -> Self {
        Self {
            source: SourceBuffer::from_str(source),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn tokenize(source: &'a str) -> Result<Vec<JsonToken>, JsonError> {
        let mut lexer = Self::from_str(source);
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
}

impl<R: Read> Lexer<R> {
    pub fn new(reader: R) -> Self {
        Self {
            source: SourceBuffer::new(reader),
        }
    }

    pub fn position(&self) -> usize {
        self.source.position()
    }

    pub fn location(&self) -> Location {
        self.source.location()
    }

    pub fn discard_consumed(&mut self) {
        self.source.discard_consumed();
    }

    fn error_at(&self, kind: JsonErrorKind, span: Span, location: Location) -> JsonError {
        JsonError::at(kind, span, location)
    }

    fn io_err(err: std::io::Error) -> JsonError {
        JsonError::io(err)
    }

    pub fn next_token(&mut self) -> Result<JsonToken, JsonError> {
        let start_loc = self.source.location();
        let pos = self.source.position();

        let Some(c) = self.source.peek_char().map_err(Self::io_err)? else {
            return Ok(JsonToken::simple(
                JsonTokenKind::Stop,
                Span {
                    start: pos,
                    end: pos + 1,
                },
                start_loc,
            ));
        };

        match c {
            '{' => self.bump_simple(JsonTokenKind::LBrace),
            '}' => self.bump_simple(JsonTokenKind::RBrace),
            '[' => self.bump_simple(JsonTokenKind::LBracket),
            ']' => self.bump_simple(JsonTokenKind::RBracket),
            ':' => self.bump_simple(JsonTokenKind::Colon),
            ',' => self.bump_simple(JsonTokenKind::Comma),
            ' ' | '\n' | '\r' | '\t' => self.bump_simple(JsonTokenKind::Whitespace),
            '"' => self.parse_string(start_loc),
            't' => self.parse_literal(JsonTokenKind::True, "true", start_loc),
            'f' => self.parse_literal(JsonTokenKind::False, "false", start_loc),
            'n' => self.parse_literal(JsonTokenKind::Null, "null", start_loc),
            '0'..='9' | '-' => self.parse_number(start_loc),
            _ => {
                let start = self.source.position();
                self.source.bump_char().map_err(Self::io_err)?;
                Err(self.error_at(
                    JsonErrorKind::InvalidCharacter,
                    Span {
                        start,
                        end: self.source.position(),
                    },
                    start_loc,
                ))
            }
        }
    }

    pub fn next_significant(&mut self) -> Result<JsonToken, JsonError> {
        loop {
            let token = self.next_token()?;
            match token.kind {
                JsonTokenKind::Whitespace => {
                    self.discard_consumed();
                    continue;
                }
                _ => {
                    self.discard_consumed();
                    return Ok(token);
                }
            }
        }
    }

    fn bump_simple(&mut self, kind: JsonTokenKind) -> Result<JsonToken, JsonError> {
        let location = self.source.location();
        let start = self.source.position();
        self.source.bump_char().map_err(Self::io_err)?;
        Ok(JsonToken::simple(
            kind,
            Span {
                start,
                end: self.source.position(),
            },
            location,
        ))
    }

    fn parse_literal(
        &mut self,
        token_type: JsonTokenKind,
        expected: &'static str,
        start_loc: Location,
    ) -> Result<JsonToken, JsonError> {
        let start = self.source.position();
        for ch in expected.chars() {
            match self.source.peek_char().map_err(Self::io_err)? {
                Some(c) if c == ch => {
                    self.source.bump_char().map_err(Self::io_err)?;
                }
                _ => {
                    return Err(self.error_at(
                        JsonErrorKind::InvalidLiteral { expected },
                        Span {
                            start,
                            end: self
                                .source
                                .position()
                                .max(start)
                                .min(start + expected.len()),
                        },
                        start_loc,
                    ));
                }
            }
        }

        // Literals must be complete tokens (not prefixes of identifiers).
        if let Some(next) = self.source.peek_char().map_err(Self::io_err)? {
            if next.is_ascii_alphanumeric() || next == '_' {
                return Err(self.error_at(
                    JsonErrorKind::InvalidLiteral { expected },
                    Span {
                        start,
                        end: self.source.position(),
                    },
                    start_loc,
                ));
            }
        }

        Ok(JsonToken::simple(
            token_type,
            Span {
                start,
                end: self.source.position(),
            },
            start_loc,
        ))
    }

    fn has_leading_zero(raw: &str) -> bool {
        let int_part = raw.strip_prefix('-').unwrap_or(raw);
        matches!(int_part.as_bytes(), [b'0', b'0'..=b'9', ..])
    }

    fn parse_number(&mut self, start_loc: Location) -> Result<JsonToken, JsonError> {
        let start = self.source.position();
        let mut state = ParseNumberState::Start;
        let mut stopped_on_error = false;

        loop {
            let Some(c) = self.source.peek_char().map_err(Self::io_err)? else {
                break;
            };

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
                ParseNumberState::OnSeparator => break,
                ParseNumberState::OnError => {
                    self.source.bump_char().map_err(Self::io_err)?;
                    stopped_on_error = true;
                    break;
                }
                _ => {
                    state = next_state;
                    self.source.bump_char().map_err(Self::io_err)?;
                }
            }
        }

        let end = self.source.position();
        let span = Span { start, end };

        if stopped_on_error || end == start {
            return Err(self.error_at(JsonErrorKind::InvalidNumber, span, start_loc));
        }

        let raw = std::str::from_utf8(self.source.slice_abs(start, end))
            .map_err(|_| self.error_at(JsonErrorKind::InvalidNumber, span, start_loc))?;

        if Self::has_leading_zero(raw) {
            return Err(self.error_at(JsonErrorKind::InvalidNumber, span, start_loc));
        }

        match state {
            ParseNumberState::InDecimal
            | ParseNumberState::Integer
            | ParseNumberState::ExpValue => {
                let value = raw
                    .parse::<f32>()
                    .map_err(|_| self.error_at(JsonErrorKind::InvalidNumber, span, start_loc))?;
                Ok(JsonToken::number(span, start_loc, value))
            }
            _ => Err(self.error_at(JsonErrorKind::InvalidNumber, span, start_loc)),
        }
    }

    fn must_be_escaped(ch: char) -> bool {
        let mut buf = [0u8; 4];
        let encoded: &str = ch.encode_utf8(&mut buf);
        MUST_BE_ESCAPED.contains(&encoded)
    }

    fn peek_bytes(&mut self, len: usize) -> Result<Option<Vec<u8>>, JsonError> {
        self.source.ensure(len).map_err(Self::io_err)?;
        let start = self.source.position();
        let end = start + len;
        if end > self.source.buffered_end() {
            return Ok(None);
        }
        Ok(Some(self.source.slice_abs(start, end).to_vec()))
    }

    fn parse_string(&mut self, start_loc: Location) -> Result<JsonToken, JsonError> {
        let start = self.source.position();
        self.source.bump_char().map_err(Self::io_err)?; // opening '"'

        loop {
            let Some(c) = self.source.peek_char().map_err(Self::io_err)? else {
                return Err(self.error_at(
                    JsonErrorKind::UnterminatedString,
                    Span {
                        start,
                        end: self.source.position(),
                    },
                    start_loc,
                ));
            };

            if c == '\\' {
                let esc_pos = self.source.position();
                if let Some(bytes) = self.peek_bytes(2)? {
                    if let Ok(escape) = std::str::from_utf8(&bytes) {
                        if ESCAPE_TOKENS.contains(&escape) {
                            self.source.bump_char().map_err(Self::io_err)?;
                            self.source.bump_char().map_err(Self::io_err)?;
                            continue;
                        }
                    }
                }
                if let Some(bytes) = self.peek_bytes(6)? {
                    if let Ok(escape) = std::str::from_utf8(&bytes) {
                        if Self::is_hex_escape(escape) {
                            for _ in 0..6 {
                                self.source.bump_char().map_err(Self::io_err)?;
                            }
                            continue;
                        }
                    }
                }

                let starts_u = self
                    .peek_bytes(2)?
                    .as_deref()
                    .and_then(|b| std::str::from_utf8(b).ok())
                    == Some("\\u");
                if starts_u {
                    let end = (esc_pos + 6).min(self.source.buffered_end().max(esc_pos + 2));
                    return Err(self.error_at(
                        JsonErrorKind::InvalidUnicodeEscape,
                        Span { start, end },
                        start_loc,
                    ));
                }

                let end = (esc_pos + 2).min(self.source.buffered_end().max(esc_pos + 1));
                return Err(self.error_at(
                    JsonErrorKind::InvalidEscape,
                    Span { start, end },
                    start_loc,
                ));
            } else if c == '"' {
                self.source.bump_char().map_err(Self::io_err)?;
                let end = self.source.position();
                let span = Span { start, end };
                let raw = std::str::from_utf8(self.source.slice_abs(start + 1, end - 1))
                    .map_err(|_| self.error_at(JsonErrorKind::InvalidEscape, span, start_loc))?;
                let value = unescape_json_string(raw)
                    .map_err(|err| self.error_at(Self::unescape_kind(err), span, start_loc))?;
                return Ok(JsonToken::string(span, start_loc, value));
            } else if Self::must_be_escaped(c) {
                self.source.bump_char().map_err(Self::io_err)?;
                return Err(self.error_at(
                    JsonErrorKind::UnescapedControl,
                    Span {
                        start,
                        end: self.source.position(),
                    },
                    start_loc,
                ));
            } else {
                self.source.bump_char().map_err(Self::io_err)?;
            }
        }
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

    fn unescape_kind(err: UnescapeError) -> JsonErrorKind {
        match err {
            UnescapeError::InvalidEscape(_) => JsonErrorKind::InvalidEscape,
            UnescapeError::InvalidUnicodeEscape => JsonErrorKind::InvalidUnicodeEscape,
            UnescapeError::InvalidSurrogatePair => JsonErrorKind::InvalidSurrogatePair,
            UnescapeError::UnescapedControlCharacter => JsonErrorKind::UnescapedControl,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Span;
    use crate::json::{
        error::{JsonError, JsonErrorKind},
        lexer::Lexer,
        token::{JsonLexeme, JsonToken, JsonTokenKind},
    };

    fn run(source: &str) -> Result<Vec<JsonToken>, JsonError> {
        let tokens = Lexer::tokenize(source)?;
        for token in &tokens {
            println!("{}", token.display());
        }
        Ok(tokens)
    }

    fn tok(kind: JsonTokenKind, start: usize, end: usize) -> JsonToken {
        use crate::Location;
        JsonToken::simple(
            kind,
            Span { start, end },
            Location {
                line: 1,
                column: start + 1,
            },
        )
    }

    fn kinds(tokens: &[JsonToken]) -> Vec<JsonTokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    fn assert_kinds_spans(source: &str, expected: &[(JsonTokenKind, usize, usize)]) {
        let tokens = run(source).expect("should lex");
        assert_eq!(tokens.len(), expected.len(), "source {:?}", source);
        for (token, (kind, start, end)) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&token.kind, kind, "kind {:?}", source);
            assert_eq!(
                token.span,
                Span {
                    start: *start,
                    end: *end
                },
                "span {:?}",
                source
            );
        }
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
            assert_kinds_spans(s, &[(JsonTokenKind::String, start, end)]);
        }
    }

    #[test]
    fn strings_unicode_escape() {
        let input = "\"\\u0041\"";
        assert_kinds_spans(input, &[(JsonTokenKind::String, 0, input.len())]);
        let tokens = run(input).unwrap();
        assert_eq!(tokens[0].lexeme, JsonLexeme::String("A".into()));

        let input = "\"\\uD83D\\uDE00\"";
        assert_kinds_spans(input, &[(JsonTokenKind::String, 0, input.len())]);
        let tokens = run(input).unwrap();
        assert_eq!(tokens[0].lexeme, JsonLexeme::String("😀".into()));
    }

    #[test]
    fn strings_raw_unicode() {
        let cases: [&str; 5] = ["\"é\"", "\"€\"", "\"🙂\"", "\"café\"", "\"é\\n🙂\""];
        for s in cases {
            assert_kinds_spans(s, &[(JsonTokenKind::String, 0, s.len())]);
        }
    }

    #[test]
    fn unicode_string_keeps_following_byte_spans() {
        assert_kinds_spans(
            "\"é\",1",
            &[
                (JsonTokenKind::String, 0, 4),
                (JsonTokenKind::Comma, 4, 5),
                (JsonTokenKind::Number, 5, 6),
            ],
        );

        assert_kinds_spans(
            "{\"a\":\"é\"}",
            &[
                (JsonTokenKind::LBrace, 0, 1),
                (JsonTokenKind::String, 1, 4),
                (JsonTokenKind::Colon, 4, 5),
                (JsonTokenKind::String, 5, 9),
                (JsonTokenKind::RBrace, 9, 10),
            ],
        );

        assert_kinds_spans(
            "[\"🙂\"]",
            &[
                (JsonTokenKind::LBracket, 0, 1),
                (JsonTokenKind::String, 1, 7),
                (JsonTokenKind::RBracket, 7, 8),
            ],
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
            assert_kinds_spans(s, &[(JsonTokenKind::String, 0, s.len())]);
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
            assert_kinds_spans(s, &[(JsonTokenKind::Number, 0, s.len())]);
        }
    }

    #[test]
    fn numbers_with_fraction_and_exponent() {
        let input: [&str; 4] = ["1.2e3", "1.2E+3", "-1.2e-3", "0.0e0"];
        for s in input {
            assert_kinds_spans(s, &[(JsonTokenKind::Number, 0, s.len())]);
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

    #[test]
    fn lexical_errors_carry_kind_and_span() {
        let err = run("@").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidCharacter);
        assert_eq!(err.span, Some(Span { start: 0, end: 1 }));
        assert_eq!(
            err.to_string(),
            "lexical error at 1:1 (0..1): invalid character"
        );

        let err = run("tru").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidLiteral { expected: "true" });
        assert_eq!(err.span, Some(Span { start: 0, end: 3 }));
        assert_eq!(
            err.to_string(),
            "lexical error at 1:1 (0..3): invalid literal, expected true"
        );

        let err = run("01").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidNumber);
        assert_eq!(err.span, Some(Span { start: 0, end: 2 }));

        let err = run("1.").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidNumber);
        assert_eq!(err.span, Some(Span { start: 0, end: 2 }));

        let err = run("\"abc").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::UnterminatedString);
        assert_eq!(err.span, Some(Span { start: 0, end: 4 }));

        let err = run("\"\\x\"").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidEscape);
        assert_eq!(err.span, Some(Span { start: 0, end: 3 }));

        let err = run("\"\\uZZZZ\"").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidUnicodeEscape);
        assert_eq!(err.span, Some(Span { start: 0, end: 7 }));

        let err = run("\"\n\"").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::UnescapedControl);
        assert_eq!(err.span, Some(Span { start: 0, end: 2 }));
    }

    #[test]
    fn lone_surrogate_is_lexical_error() {
        let err = run("\"\\uD800\"").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidSurrogatePair);
    }
}
