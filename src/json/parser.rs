use std::collections::HashMap;

use crate::json::{
    error::{ExpectedSyntax, JsonError, JsonErrorKind},
    escape::{UnescapeError, unescape_json_string},
    lexer::Lexer,
    token::{JsonToken, JsonTokenKind},
    value::JsonValue,
};
use crate::source::Span;

#[derive(Debug, Clone)]
pub struct JsonParser<'a> {
    current_token_idx: usize,
    source: &'a str,
    tokens: &'a [JsonToken],
}

impl<'a> JsonParser<'a> {
    pub fn parse(source: &str) -> Result<JsonValue, JsonError> {
        let tokens = Lexer::tokenize(source)?;
        if tokens.is_empty() {
            return Err(JsonError::new(
                JsonErrorKind::EmptyInput,
                Span {
                    start: 0,
                    end: source.len(),
                },
            ));
        }

        let mut parser = JsonParser::new(&tokens, source);
        let value = parser.parse_value()?;
        parser.expect_eof()?;

        Ok(value)
    }

    pub fn new(tokens: &'a [JsonToken], source: &'a str) -> Self {
        Self {
            current_token_idx: 0,
            source,
            tokens,
        }
    }

    fn expect_eof(&self) -> Result<(), JsonError> {
        let next = self.current_token_idx + 1;
        if next < self.tokens.len() {
            return Err(JsonError::new(
                JsonErrorKind::TrailingInput,
                self.tokens[next].span,
            ));
        }
        Ok(())
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.parse_token(JsonTokenKind::LBrace)?;
        self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBrace))?;
        let members = self.parse_members()?;
        self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBrace))?;
        self.parse_token(JsonTokenKind::RBrace)?;
        Ok(JsonValue::object(HashMap::<String, JsonValue>::from_iter(
            members.into_iter(),
        )))
    }

    fn parse_members(&mut self) -> Result<Vec<(String, JsonValue)>, JsonError> {
        let mut members: Vec<(String, JsonValue)> = vec![];
        loop {
            if self.get_token().kind != JsonTokenKind::String {
                if !members.is_empty() {
                    return Err(self.unexpected(ExpectedSyntax::Token(JsonTokenKind::String)));
                }
                self.back_token();
                break;
            }

            members.push(self.parse_pair()?);
            self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBrace))?;
            if self.parse_token(JsonTokenKind::Comma).is_err() {
                break;
            }
            self.next_token(ExpectedSyntax::Token(JsonTokenKind::String))?;
        }
        Ok(members)
    }

    fn parse_pair(&mut self) -> Result<(String, JsonValue), JsonError> {
        let key_token = self.parse_token(JsonTokenKind::String)?;
        let key = self.get_string_content(&key_token)?;
        self.next_token(ExpectedSyntax::Token(JsonTokenKind::Colon))?;
        self.parse_token(JsonTokenKind::Colon)?;
        self.next_token(ExpectedSyntax::Value)?;
        let value = self.parse_value()?;
        Ok((key.to_string(), value))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.parse_token(JsonTokenKind::LBracket)?;
        self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBracket))?;
        let elements = self.parse_elements()?;
        self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBracket))?;
        self.parse_token(JsonTokenKind::RBracket)?;
        Ok(JsonValue::array(elements))
    }

    fn parse_elements(&mut self) -> Result<Vec<JsonValue>, JsonError> {
        let mut items: Vec<JsonValue> = vec![];
        loop {
            if !Self::starts_value(&self.get_token().kind) {
                if !items.is_empty() {
                    return Err(self.unexpected(ExpectedSyntax::Value));
                }
                self.back_token();
                break;
            }

            items.push(self.parse_value()?);
            self.next_token(ExpectedSyntax::Token(JsonTokenKind::RBracket))?;
            if self.parse_token(JsonTokenKind::Comma).is_err() {
                break;
            }
            self.next_token(ExpectedSyntax::Value)?;
        }
        Ok(items)
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        let first = self.get_token();

        match first.kind {
            JsonTokenKind::Null => Ok(JsonValue::null()),
            JsonTokenKind::True => Ok(JsonValue::boolean(true)),
            JsonTokenKind::False => Ok(JsonValue::boolean(false)),
            JsonTokenKind::String => Ok(JsonValue::string(self.get_string_content(first)?)),
            JsonTokenKind::Number => Ok(JsonValue::number(self.get_number_content(first)?)),
            JsonTokenKind::LBracket => self.parse_array(),
            JsonTokenKind::LBrace => self.parse_object(),
            _ => Err(self.unexpected(ExpectedSyntax::Value)),
        }
    }

    fn parse_token(&mut self, kind: JsonTokenKind) -> Result<JsonToken, JsonError> {
        let token = self.get_token().clone();
        if token.kind != kind {
            self.back_token();
            return Err(JsonError::new(
                JsonErrorKind::UnexpectedToken {
                    expected: ExpectedSyntax::Token(kind),
                    found: token.kind,
                },
                token.span,
            ));
        }

        Ok(token)
    }

    fn next_token(&mut self, expected: ExpectedSyntax) -> Result<usize, JsonError> {
        if self.current_token_idx + 1 >= self.tokens.len() {
            let end = self
                .tokens
                .last()
                .map(|token| token.span.end)
                .unwrap_or(self.source.len());
            return Err(JsonError::new(
                JsonErrorKind::UnexpectedEof { expected },
                Span { start: end, end },
            ));
        }

        self.current_token_idx += 1;
        Ok(self.current_token_idx)
    }

    fn back_token(&mut self) {
        debug_assert!(
            self.current_token_idx > 0,
            "cannot move before the first token"
        );
        if self.current_token_idx > 0 {
            self.current_token_idx -= 1;
        }
    }

    fn unexpected(&self, expected: ExpectedSyntax) -> JsonError {
        let token = self.get_token();
        JsonError::new(
            JsonErrorKind::UnexpectedToken {
                expected,
                found: token.kind.clone(),
            },
            token.span,
        )
    }

    fn starts_value(kind: &JsonTokenKind) -> bool {
        matches!(
            kind,
            JsonTokenKind::Null
                | JsonTokenKind::True
                | JsonTokenKind::False
                | JsonTokenKind::String
                | JsonTokenKind::Number
                | JsonTokenKind::LBracket
                | JsonTokenKind::LBrace
        )
    }

    fn get_token(&self) -> &'a JsonToken {
        self.tokens.get(self.current_token_idx).unwrap()
    }

    fn get_string_content(&self, token: &JsonToken) -> Result<String, JsonError> {
        debug_assert_eq!(token.kind, JsonTokenKind::String);
        let content = self
            .source
            .get(token.span.start + 1..token.span.end - 1)
            .expect("string token span includes quotes");
        unescape_json_string(content)
            .map_err(|err| JsonError::new(Self::unescape_kind(err), token.span))
    }

    fn unescape_kind(err: UnescapeError) -> JsonErrorKind {
        match err {
            UnescapeError::InvalidEscape(_) => JsonErrorKind::InvalidEscape,
            UnescapeError::InvalidUnicodeEscape => JsonErrorKind::InvalidUnicodeEscape,
            UnescapeError::InvalidSurrogatePair => JsonErrorKind::InvalidSurrogatePair,
            UnescapeError::UnescapedControlCharacter => JsonErrorKind::UnescapedControl,
        }
    }

    fn get_number_content(&self, token: &JsonToken) -> Result<f32, JsonError> {
        debug_assert_eq!(token.kind, JsonTokenKind::Number);
        let content = self
            .source
            .get(token.span.start..token.span.end)
            .expect("number token span is inside the source");
        content
            .parse::<f32>()
            .map_err(|_| JsonError::new(JsonErrorKind::InvalidNumber, token.span))
    }
}

#[cfg(test)]
mod test {
    use crate::json::{
        JsonValue,
        error::{ExpectedSyntax, JsonError, JsonErrorKind},
        parser::JsonParser,
        token::{JsonTokenKind},
    };
    use crate::source::Span;
    use std::collections::HashMap;

    fn parse(source: &str) -> Result<JsonValue, JsonError> {
        JsonParser::parse(source)
    }

    fn string(text: &str) -> JsonValue {
        JsonValue::String(text.to_string())
    }

    fn number(n: f32) -> JsonValue {
        JsonValue::Number(n)
    }

    fn array(items: Vec<JsonValue>) -> JsonValue {
        JsonValue::Array(items)
    }

    fn object(pairs: &[(&str, JsonValue)]) -> JsonValue {
        JsonValue::Object(HashMap::from_iter(
            pairs.iter().map(|(k, v)| ((*k).to_string(), v.clone())),
        ))
    }

    #[test]
    fn literals() {
        let input: [(&str, JsonValue); 3] = [
            ("true", JsonValue::True),
            ("false", JsonValue::False),
            ("null", JsonValue::Null),
        ];
        for (s, expected) in input {
            assert_eq!(parse(s), Ok(expected), "literal {:?}", s);
        }
    }

    #[test]
    fn whitespace_is_insignificant() {
        let padded: [(&str, JsonValue); 4] = [
            ("  true  ", JsonValue::True),
            ("\tfalse\t", JsonValue::False),
            ("\nnull\n", JsonValue::Null),
            ("\r1\r", number(1.0)),
        ];
        for (s, expected) in padded {
            assert_eq!(parse(s), Ok(expected), "padded {:?}", s);
        }

        let mixed = " \t\r\n\"ab\"";
        assert_eq!(parse(mixed), Ok(string("ab")));
        assert_eq!(
            parse(" [ 1 , 2 ] "),
            Ok(array(vec![number(1.0), number(2.0)]))
        );
        assert_eq!(
            parse(" { \"a\" : true } "),
            Ok(object(&[("a", JsonValue::True)]))
        );
    }

    #[test]
    fn numbers() {
        let input: [(&str, f32); 16] = [
            ("0", 0.0),
            ("-0", -0.0),
            ("1", 1.0),
            ("10", 10.0),
            ("-1", -1.0),
            ("-10", -10.0),
            ("0.5", 0.5),
            ("10.0", 10.0),
            ("3.25", 3.25),
            ("-1.5", -1.5),
            ("1e2", 100.0),
            ("1E2", 100.0),
            ("1e+2", 100.0),
            ("1e-1", 1e-1),
            ("1.5e1", 15.0),
            ("-2.5E+1", -25.0),
        ];
        for (s, expected) in input {
            assert_eq!(parse(s), Ok(number(expected)), "number {:?}", s);
        }
    }

    #[test]
    fn strings() {
        let input: [(&str, &str); 6] = [
            (r#""""#, ""),
            (r#""hello""#, "hello"),
            (r#""hello world""#, "hello world"),
            (r#"" ""#, " "),
            (r#""é""#, "é"),
            (r#""🙂""#, "🙂"),
        ];
        for (s, text) in input {
            assert_eq!(parse(s), Ok(string(text)), "string {:?}", s);
        }
    }

    #[test]
    fn string_escape_sequences() {
        let input: [(&str, &str); 13] = [
            (r#""\"""#, "\""),
            (r#""\\""#, "\\"),
            (r#""\/""#, "/"),
            (r#""\b""#, "\u{0008}"),
            (r#""\f""#, "\u{000c}"),
            (r#""\n""#, "\n"),
            (r#""\r""#, "\r"),
            (r#""\t""#, "\t"),
            (r#""a\/b""#, "a/b"),
            (r#""a\nb\tc""#, "a\nb\tc"),
            (r#""\u0041""#, "A"),
            (r#""\u00E9""#, "é"),
            (r#""\u20AC""#, "€"),
        ];
        for (s, text) in input {
            assert_eq!(parse(s), Ok(string(text)), "escape {:?}", s);
        }
    }

    #[test]
    fn string_surrogate_pair() {
        assert_eq!(parse(r#""\uD83D\uDE00""#), Ok(string("😀")));
    }

    #[test]
    fn empty_containers() {
        assert_eq!(parse("[]"), Ok(array(vec![])));
        assert_eq!(parse("{}"), Ok(object(&[])));
        assert_eq!(parse("[ ]"), Ok(array(vec![])));
        assert_eq!(parse("{ }"), Ok(object(&[])));
    }

    #[test]
    fn arrays() {
        assert_eq!(parse("[1]"), Ok(array(vec![number(1.0)])));
        assert_eq!(
            parse("[1,2,3]"),
            Ok(array(vec![number(1.0), number(2.0), number(3.0)]))
        );
        assert_eq!(
            parse("[true,false,null]"),
            Ok(array(vec![
                JsonValue::True,
                JsonValue::False,
                JsonValue::Null,
            ]))
        );
        assert_eq!(
            parse(r#"["x","y"]"#),
            Ok(array(vec![string("x"), string("y")]))
        );
        assert_eq!(parse("[[]]"), Ok(array(vec![array(vec![])])));
        assert_eq!(
            parse("[1,[2,3]]"),
            Ok(array(vec![
                number(1.0),
                array(vec![number(2.0), number(3.0)]),
            ]))
        );
    }

    #[test]
    fn objects() {
        assert_eq!(parse(r#"{"a":1}"#), Ok(object(&[("a", number(1.0))])));
        assert_eq!(
            parse(r#"{"a":1,"b":true}"#),
            Ok(object(&[("a", number(1.0)), ("b", JsonValue::True)]))
        );
        assert_eq!(parse(r#"{"":0}"#), Ok(object(&[("", number(0.0))])));
        assert_eq!(
            parse(r#"{"s":"hi","n":null,"f":false}"#),
            Ok(object(&[
                ("s", string("hi")),
                ("n", JsonValue::Null),
                ("f", JsonValue::False),
            ]))
        );
        assert_eq!(
            parse(" { \"a\" : 1 , \"b\" : 2 } "),
            Ok(object(&[("a", number(1.0)), ("b", number(2.0))]))
        );
    }

    #[test]
    fn nested_values() {
        assert_eq!(
            parse(r#"{"arr":[1,{"k":null}],"ok":true}"#),
            Ok(object(&[
                (
                    "arr",
                    array(vec![number(1.0), object(&[("k", JsonValue::Null)]),]),
                ),
                ("ok", JsonValue::True),
            ]))
        );
        assert_eq!(
            parse(r#"[{"a":[true]},{"b":"x"}]"#),
            Ok(array(vec![
                object(&[("a", array(vec![JsonValue::True]))]),
                object(&[("b", string("x"))]),
            ]))
        );
    }

    #[test]
    fn deeply_nested_values() {
        let mut source = String::from("1");
        let mut expected = number(1.0);
        for _ in 0..8 {
            source = format!("[{source}]");
            expected = array(vec![expected]);
        }
        assert_eq!(parse(&source), Ok(expected));

        let mut source = String::from("null");
        let mut expected = JsonValue::Null;
        for _ in 0..8 {
            source = format!(r#"{{"k":{source}}}"#);
            expected = object(&[("k", expected)]);
        }
        assert_eq!(parse(&source), Ok(expected));
    }

    #[test]
    fn empty_input_is_error() {
        let input: [&str; 6] = ["", " ", "\t", "\n", "\r", " \t\r\n"];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn trailing_commas_are_errors() {
        let input: [&str; 6] = [
            "[1,]",
            "[1,2,]",
            "[,]",
            "{\"a\":1,}",
            "{\"a\":1,\"b\":2,}",
            "{,}",
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn missing_separators_are_errors() {
        let input: [&str; 8] = [
            "[1 2]",
            "[1,,2]",
            "[,1]",
            r#"{"a" 1}"#,
            r#"{"a":1 "b":2}"#,
            r#"{"a":1,,"b":2}"#,
            r#"{,"a":1}"#,
            r#"{"a"::1}"#,
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn incomplete_containers_are_errors() {
        let input: [&str; 10] = [
            "[",
            "{",
            "[1",
            "[1,",
            r#"{"a""#,
            r#"{"a":"#,
            r#"{"a":1"#,
            r#"{"a":}"#,
            r#"{:1}"#,
            "]",
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn mismatched_delimiters_are_errors() {
        let input: [&str; 6] = ["[}", "{]", "[1}", "{1]", r#"{"a":1]"#, r#"["a":1]"#];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn object_keys_must_be_strings() {
        let input: [&str; 6] = [
            "{true:1}",
            "{false:1}",
            "{null:1}",
            "{1:1}",
            "{[]:1}",
            "{{}:1}",
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn extra_tokens_are_errors() {
        let input: [&str; 7] = [
            "true false",
            "1 2",
            "null null",
            r#"{"a":1}{"b":2}"#,
            "[1][2]",
            r#""a""b""#,
            "true,",
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn documents_outside_json_are_errors() {
        let input: [&str; 12] = [
            "NaN",
            "Infinity",
            "-Infinity",
            "undefined",
            "+1",
            "01",
            "'abc'",
            "// comment",
            "/* comment */",
            r#""\uD800""#,
            r#""\uDE00""#,
            "\u{FEFF}true",
        ];
        for s in input {
            assert!(parse(s).is_err(), "expected parse error for {:?}", s);
        }
    }

    #[test]
    fn parse_errors_carry_kind_and_span() {
        let err = parse("").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::EmptyInput);
        assert_eq!(err.span, Some(Span { start: 0, end: 0 }));
        assert_eq!(err.to_string(), "parse error at 0..0: empty input");

        let err = parse(" ").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::EmptyInput);
        assert_eq!(err.span, Some(Span { start: 0, end: 1 }));

        let err = parse("{").unwrap_err();
        assert_eq!(
            err.kind,
            JsonErrorKind::UnexpectedEof {
                expected: ExpectedSyntax::Token(JsonTokenKind::RBrace),
            }
        );
        assert_eq!(err.span, Some(Span { start: 1, end: 1 }));
        assert_eq!(
            err.to_string(),
            "parse error at 1..1: expected rbrace but reached end of input"
        );

        let err = parse("true false").unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::TrailingInput);
        assert_eq!(err.span, Some(Span { start: 5, end: 10 }));
        assert_eq!(err.to_string(), "parse error at 5..10: trailing input");

        let err = parse("[1}").unwrap_err();
        assert_eq!(
            err.kind,
            JsonErrorKind::UnexpectedToken {
                expected: ExpectedSyntax::Token(JsonTokenKind::RBracket),
                found: JsonTokenKind::RBrace,
            }
        );
        assert_eq!(err.span, Some(Span { start: 2, end: 3 }));
        assert_eq!(
            err.to_string(),
            "parse error at 2..3: expected rbracket but found rbrace"
        );

        let err = parse(r#"{"a":}"#).unwrap_err();
        assert_eq!(
            err.kind,
            JsonErrorKind::UnexpectedToken {
                expected: ExpectedSyntax::Value,
                found: JsonTokenKind::RBrace,
            }
        );

        let err = parse(r#""\uD800""#).unwrap_err();
        assert_eq!(err.kind, JsonErrorKind::InvalidSurrogatePair);
        assert_eq!(err.span, Some(Span { start: 0, end: 8 }));
        assert_eq!(
            err.to_string(),
            "lexical error at 0..8: invalid surrogate pair"
        );
    }
}
