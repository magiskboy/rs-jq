use std::collections::HashMap;

use crate::json::{
    error::JsonParserError,
    escape::unescape_json_string,
    lexer::Lexer,
    token::{JsonToken, JsonTokenKind},
    value::JsonValue,
};

#[derive(Debug, Clone)]
pub struct JsonParser<'a> {
    current_token_idx: usize,
    source: &'a str,
    tokens: &'a [JsonToken],
}

impl<'a> JsonParser<'a> {
    pub fn parse(source: &str) -> Result<JsonValue, JsonParserError> {
        let tokens = Lexer::tokenize(source)?;
        if tokens.is_empty() {
            return Err(JsonParserError::ParserError(String::from("json is empty")));
        }

        let mut parser = JsonParser::new(&tokens, source);
        let value = parser.parse_value()?;
        parser.expect_eof()?;

        Ok(value)
    }

    pub fn expect_eof(&self) -> Result<(), JsonParserError> {
        if self.current_token_idx < self.tokens.len() - 1 {
            return Err(JsonParserError::ParserError(String::from("expect oef")));
        }
        Ok(())
    }

    pub fn new(tokens: &'a [JsonToken], source: &'a str) -> Self {
        Self {
            current_token_idx: 0,
            source,
            tokens,
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParserError> {
        self.parse_token(JsonTokenKind::LBrace)?;
        self.next_token()?;
        let members = self.parse_members()?;
        self.next_token()?;
        self.parse_token(JsonTokenKind::RBrace)?;
        Ok(JsonValue::Object(HashMap::<String, JsonValue>::from_iter(
            members.into_iter(),
        )))
    }

    fn parse_members(&mut self) -> Result<Vec<(String, JsonValue)>, JsonParserError> {
        let mut members: Vec<(String, JsonValue)> = vec![];
        loop {
            if let Ok(pair) = self.parse_pair() {
                members.push(pair);
                self.next_token()?;
                if let Ok(_) = self.parse_token(JsonTokenKind::Comma) {
                    self.next_token()?;
                    continue;
                } else {
                    break;
                }
            } else {
                if members.len() > 0 {
                    return Err(JsonParserError::ParserError(String::from("invalid object")));
                }
                // if parse_pair is fail, token was back by them so we don't need back at here
                // self.back_token()?;
                break;
            }
        }
        Ok(members)
    }

    fn parse_pair(&mut self) -> Result<(String, JsonValue), JsonParserError> {
        let key_token = self.parse_token(JsonTokenKind::String)?;
        let key = self.get_string_content(&key_token)?;
        self.next_token()?;
        self.parse_token(JsonTokenKind::Colon)?;
        self.next_token()?;
        let value = self.parse_value()?;
        Ok((key.to_string(), value))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonParserError> {
        self.parse_token(JsonTokenKind::LBracket)?;
        self.next_token()?;
        let elements = self.parse_elements()?;
        self.next_token()?;
        self.parse_token(JsonTokenKind::RBracket)?;
        Ok(JsonValue::Array(elements))
    }

    fn parse_elements(&mut self) -> Result<Vec<JsonValue>, JsonParserError> {
        let mut items: Vec<JsonValue> = vec![];
        loop {
            if let Ok(item) = self.parse_value() {
                items.push(item);
                self.next_token()?;
                if let Ok(_) = self.parse_token(JsonTokenKind::Comma) {
                    self.next_token()?;
                    continue;
                } else {
                    break;
                }
            } else {
                if items.len() > 0 {
                    return Err(JsonParserError::ParserError(String::from("invalid array")));
                }
                self.back_token()?;
                break;
            }
        }
        Ok(items)
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonParserError> {
        let first = self.get_token();

        match first.kind {
            JsonTokenKind::Null => Ok(JsonValue::Null),
            JsonTokenKind::True => Ok(JsonValue::True),
            JsonTokenKind::False => Ok(JsonValue::False),
            JsonTokenKind::String => Ok(JsonValue::String(self.get_string_content(first)?)),
            JsonTokenKind::Number => Ok(JsonValue::Number(self.get_number_content(first)?)),
            JsonTokenKind::LBracket => self.parse_array(),
            JsonTokenKind::LBrace => self.parse_object(),
            _ => Err(JsonParserError::ParserError(String::from(
                "Unexpected token, position is 0",
            ))),
        }
    }

    fn parse_token(&mut self, kind: JsonTokenKind) -> Result<JsonToken, JsonParserError> {
        let token = self.get_token();
        if token.kind != kind {
            let mut msg = String::new();
            msg.push_str("Unexpected token, expect ");
            msg.push_str(kind.to_string().as_str());
            msg.push_str(" but ");
            msg.push_str(token.kind.to_string().as_str());
            self.back_token()?;

            return Err(JsonParserError::ParserError(msg));
        }

        Ok(token.clone())
    }

    fn next_token(&mut self) -> Result<usize, JsonParserError> {
        if self.current_token_idx == self.tokens.len() - 1 {
            return Err(JsonParserError::ParserError(String::from(
                "[JsonParser.next_token] Index is out of range",
            )));
        }

        self.current_token_idx = self.current_token_idx + 1;
        Ok(self.current_token_idx)
    }

    fn back_token(&mut self) -> Result<usize, JsonParserError> {
        if self.current_token_idx == 0 {
            return Err(JsonParserError::ParserError(String::from(
                "[JsonParser.back_token] Index is out of range",
            )));
        }
        self.current_token_idx = self.current_token_idx - 1;
        Ok(self.current_token_idx)
    }

    fn get_token(&self) -> &'a JsonToken {
        self.tokens.get(self.current_token_idx).unwrap()
    }

    fn get_string_content(&self, token: &JsonToken) -> Result<String, JsonParserError> {
        if token.kind == JsonTokenKind::String {
            let content = self
                .source
                .get(token.span.start + 1..token.span.end - 1) // ignore wrapper \" and \"
                .ok_or(JsonParserError::ParserError(String::from(
                    "Can't get content of string",
                )))?;
            let escaped = unescape_json_string(content)
                .map_err(|_| JsonParserError::ParserError(String::from("Can't unescape string")))?;
            return Ok(String::from(escaped));
        } else {
            return Err(JsonParserError::ParserError(String::from(
                "Can't get content of string",
            )));
        }
    }

    fn get_number_content(&self, token: &JsonToken) -> Result<f32, JsonParserError> {
        if token.kind == JsonTokenKind::Number {
            let content = self.source.get(token.span.start..token.span.end).ok_or(
                JsonParserError::ParserError(String::from("Can't get content of number")),
            )?;
            let number = String::from(content).parse::<f32>().map_err(|_| {
                JsonParserError::ParserError(String::from("Can't parse number content to float32"))
            })?;
            return Ok(number);
        }

        Err(JsonParserError::ParserError(String::from(
            "Can't get content of number",
        )))
    }
}

#[cfg(test)]
mod test {
    use crate::json::{JsonValue, error::JsonParserError, parser::JsonParser};
    use std::collections::HashMap;

    fn parse(source: &str) -> Result<JsonValue, JsonParserError> {
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
}
