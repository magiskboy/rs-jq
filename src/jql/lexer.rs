use crate::{
    jql::{
        error::{JqlError, JqlErrorKind},
        token::{JqlToken, JqlTokenKind},
    },
    source::{Source, Span},
};

#[derive(Debug, Clone)]
pub struct JqlLexer<'a> {
    pub index: usize,
    pub source: Source<'a>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
enum JqlLexerState {
    Start,
    InLBrace,
    InRBrace,
    InPipe,
    InF,
    InFa,
    InFal,
    InFals,
    InFalse,
    InT,
    InTr,
    InTru,
    InTrue,
    InN,
    InNu,
    InNul,
    InNull,
    InNumber,
    OpenString,
    InString,
    CloseString,
    InIdentifier,
    StartEqual,
    InEqual,
    StartNotEqual,
    InNotEqual,
    InGreater,
    InLess,
    InGreaterEqual,
    InLessEqual,
    StartAnd,
    InAnd,
    InOr,
    Whitespace,
    InvalidToken,
}

impl<'a> JqlLexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            index: 0,
            source: Source::new(source),
        }
    }

    fn transition_table(&self, state: &JqlLexerState, c: char) -> Option<JqlLexerState> {
        let next_state = match state {
            JqlLexerState::Start => match c {
                '"' => JqlLexerState::OpenString,
                't' => JqlLexerState::InT,
                'f' => JqlLexerState::InF,
                'n' => JqlLexerState::InN,
                '&' => JqlLexerState::StartAnd,
                '!' => JqlLexerState::StartNotEqual,
                '=' => JqlLexerState::StartEqual,
                '>' => JqlLexerState::InGreater,
                '<' => JqlLexerState::InLess,
                '|' => JqlLexerState::InPipe,
                '(' => JqlLexerState::InLBrace,
                ')' => JqlLexerState::InRBrace,
                '0'..='9' => JqlLexerState::InNumber,
                '.' | 'a'..='z' | 'A'..='Z' => JqlLexerState::InIdentifier,
                ' ' => JqlLexerState::Whitespace,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InPipe => match c {
                '|' => JqlLexerState::InOr,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNumber => match c {
                '0'..='9' => JqlLexerState::InNumber,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::OpenString | JqlLexerState::InString => match c {
                '"' => JqlLexerState::CloseString,
                _ => JqlLexerState::InString,
            },
            JqlLexerState::InT => match c {
                'r' => JqlLexerState::InTr,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InTr => match c {
                'u' => JqlLexerState::InTru,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InTru => match c {
                'e' => JqlLexerState::InTrue,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InF => match c {
                'a' => JqlLexerState::InFa,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFa => match c {
                'l' => JqlLexerState::InFal,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFal => match c {
                's' => JqlLexerState::InFals,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InFals => match c {
                'e' => JqlLexerState::InFalse,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InN => match c {
                'u' => JqlLexerState::InNu,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNu => match c {
                'l' => JqlLexerState::InNul,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InNul => match c {
                'l' => JqlLexerState::InNull,
                'a'..='z' | 'A'..='Z' | '0'..='9' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InIdentifier => match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '[' | ']' => JqlLexerState::InIdentifier,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::StartEqual => JqlLexerState::InEqual,
            JqlLexerState::StartNotEqual => JqlLexerState::InNotEqual,
            JqlLexerState::InGreater => match c {
                '=' => JqlLexerState::InGreaterEqual,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::InLess => match c {
                '=' => JqlLexerState::InLessEqual,
                _ => JqlLexerState::InvalidToken,
            },
            JqlLexerState::StartAnd => JqlLexerState::InAnd,
            _ => JqlLexerState::InvalidToken,
        };

        match next_state {
            JqlLexerState::InvalidToken => None,
            _ => Some(next_state),
        }
    }

    fn accept_state(&self, state: &JqlLexerState, start: usize, end: usize) -> Option<JqlToken> {
        let kind = match state {
            JqlLexerState::InLBrace => JqlTokenKind::LParen,
            JqlLexerState::InRBrace => JqlTokenKind::RParen,
            JqlLexerState::InPipe => JqlTokenKind::Pipe,
            JqlLexerState::InFalse => JqlTokenKind::Boolean,
            JqlLexerState::InTrue => JqlTokenKind::Boolean,
            JqlLexerState::InNull => JqlTokenKind::Null,
            JqlLexerState::InNumber => JqlTokenKind::Number,
            JqlLexerState::CloseString => JqlTokenKind::String,
            JqlLexerState::InIdentifier => {
                let content = self.source.slice(start, end).unwrap();
                match Self::keyword_table(content) {
                    true => JqlTokenKind::Keyword,
                    false => JqlTokenKind::Identifier,
                }
            }
            JqlLexerState::InEqual => JqlTokenKind::EqualOp,
            JqlLexerState::InNotEqual => JqlTokenKind::NotEqualOp,
            JqlLexerState::InGreater => JqlTokenKind::GreaterOp,
            JqlLexerState::InLess => JqlTokenKind::LessOp,
            JqlLexerState::InGreaterEqual => JqlTokenKind::GreaterEqualOp,
            JqlLexerState::InLessEqual => JqlTokenKind::LessEqualOp,
            JqlLexerState::InAnd => JqlTokenKind::AndLogicalOp,
            JqlLexerState::InOr => JqlTokenKind::OrLogicalOp,
            JqlLexerState::Whitespace => JqlTokenKind::Whitespace,
            _ => JqlTokenKind::InvalidToken,
        };

        match kind {
            JqlTokenKind::InvalidToken => None,
            _ => Some(JqlToken::new(kind, start, end)),
        }
    }

    fn keyword_table(value: &str) -> bool {
        match value {
            "filter" => true,
            _ => false,
        }
    }

    fn next_token(&mut self) -> Result<JqlToken, JqlError> {
        let mut state = JqlLexerState::Start;
        let mut position = self.index;
        let mut last_accept_token: Option<JqlToken> = None;
        let mut last_accept_position = position;

        while position < self.source.len() {
            let c = self.source.char_at(position).unwrap();

            let next_state = self.transition_table(&state, c);
            if next_state.is_none() {
                break;
            }

            if let Some(token) = self.accept_state(&next_state.unwrap(), self.index, position + 1) {
                last_accept_token = Some(token);
                last_accept_position = position + 1;
            }
            state = next_state.unwrap().clone();
            position += 1;
        }

        if let Some(token) = last_accept_token {
            self.index = last_accept_position;
            if token.kind == JqlTokenKind::Whitespace {
                return self.next_token();
            }

            return Ok(token);
        }

        self.index = position + 1;
        Err(JqlError {
            kind: super::error::JqlErrorKind::GenericError,
            message: String::from("invalid token"),
        })
    }

    pub fn tokenize(source: &str) -> Result<Vec<JqlToken>, JqlError> {
        let mut lexer = JqlLexer::new(source);
        let mut tokens: Vec<JqlToken> = vec![];
        loop {
            if let Ok(token) = lexer.next_token() {
                tokens.push(token);
            } else {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod test {
    use super::JqlLexer;
    use crate::jql::error::{JqlError, JqlErrorKind};
    use crate::jql::token::{JqlToken, JqlTokenKind};

    fn run(source: &str) -> Result<Vec<JqlToken>, JqlError> {
        let tokens = JqlLexer::tokenize(source)?;
        for token in tokens.clone() {
            println!("{}", token.display(source));
        }
        Ok(tokens)
    }

    fn tok(kind: JqlTokenKind, start: usize, end: usize) -> JqlToken {
        JqlToken::new(kind, start, end)
    }

    fn kinds(tokens: &[JqlToken]) -> Vec<JqlTokenKind> {
        tokens.iter().map(|token| token.kind.clone()).collect()
    }

    fn invalid() -> JqlError {
        JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("invalid token"),
        }
    }

    fn assert_invalid(source: &str) {
        assert_eq!(run(source), Err(invalid()), "source {source:?}");
    }

    #[test]
    fn empty_and_spaces_are_insignificant() {
        for source in ["", " ", "   "] {
            assert_eq!(run(source), Ok(vec![]), "source {source:?}");
        }

        let source = "  true   false  ";
        assert_eq!(
            run(source),
            Ok(vec![
                tok(JqlTokenKind::Boolean, 2, 6),
                tok(JqlTokenKind::Boolean, 9, 14),
            ])
        );
    }

    #[test]
    fn parentheses() {
        assert_eq!(run("("), Ok(vec![tok(JqlTokenKind::LParen, 0, 1)]));
        assert_eq!(run(")"), Ok(vec![tok(JqlTokenKind::RParen, 0, 1)]));
        assert_eq!(
            run("( )"),
            Ok(vec![
                tok(JqlTokenKind::LParen, 0, 1),
                tok(JqlTokenKind::RParen, 2, 3),
            ])
        );
    }

    #[test]
    fn comparison_operators() {
        let cases = [
            ("==", JqlTokenKind::EqualOp),
            ("!=", JqlTokenKind::NotEqualOp),
            (">", JqlTokenKind::GreaterOp),
            (">=", JqlTokenKind::GreaterEqualOp),
            ("<", JqlTokenKind::LessOp),
            ("<=", JqlTokenKind::LessEqualOp),
        ];
        for (source, kind) in cases {
            assert_eq!(
                run(source),
                Ok(vec![tok(kind, 0, source.len())]),
                "operator {source:?}"
            );
        }

        assert_eq!(
            run(">>"),
            Ok(vec![
                tok(JqlTokenKind::GreaterOp, 0, 1),
                tok(JqlTokenKind::GreaterOp, 1, 2),
            ])
        );
        assert_eq!(
            run("<<"),
            Ok(vec![
                tok(JqlTokenKind::LessOp, 0, 1),
                tok(JqlTokenKind::LessOp, 1, 2),
            ])
        );
        assert_eq!(
            run(">x"),
            Ok(vec![
                tok(JqlTokenKind::GreaterOp, 0, 1),
                tok(JqlTokenKind::Identifier, 1, 2),
            ])
        );
    }

    #[test]
    fn pipe_and_logical_operators() {
        assert_eq!(run("|"), Ok(vec![tok(JqlTokenKind::Pipe, 0, 1)]));
        assert_eq!(run("||"), Ok(vec![tok(JqlTokenKind::OrLogicalOp, 0, 2)]));
        assert_eq!(run("&&"), Ok(vec![tok(JqlTokenKind::AndLogicalOp, 0, 2)]));
        assert_eq!(
            run("|||"),
            Ok(vec![
                tok(JqlTokenKind::OrLogicalOp, 0, 2),
                tok(JqlTokenKind::Pipe, 2, 3),
            ])
        );
        assert_eq!(
            run("||||"),
            Ok(vec![
                tok(JqlTokenKind::OrLogicalOp, 0, 2),
                tok(JqlTokenKind::OrLogicalOp, 2, 4),
            ])
        );
        assert_eq!(
            run("|x"),
            Ok(vec![
                tok(JqlTokenKind::Pipe, 0, 1),
                tok(JqlTokenKind::Identifier, 1, 2),
            ])
        );
        assert_eq!(
            run("true && false || null"),
            Ok(vec![
                tok(JqlTokenKind::Boolean, 0, 4),
                tok(JqlTokenKind::AndLogicalOp, 5, 7),
                tok(JqlTokenKind::Boolean, 8, 13),
                tok(JqlTokenKind::OrLogicalOp, 14, 16),
                tok(JqlTokenKind::Null, 17, 21),
            ])
        );
    }

    #[test]
    fn two_character_operators_require_the_exact_second_character() {
        for source in [
            "=", "!", "&", "=x", "!x", "&x", "===", "!==", "&&&", "&>", "|=",
        ] {
            assert_invalid(source);
        }
    }

    #[test]
    fn booleans_null_and_keyword() {
        assert_eq!(run("true"), Ok(vec![tok(JqlTokenKind::Boolean, 0, 4)]));
        assert_eq!(run("false"), Ok(vec![tok(JqlTokenKind::Boolean, 0, 5)]));
        assert_eq!(run("null"), Ok(vec![tok(JqlTokenKind::Null, 0, 4)]));
        assert_eq!(run("filter"), Ok(vec![tok(JqlTokenKind::Keyword, 0, 6)]));

        let source = "true";
        let tokens = run(source).expect("true");
        assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "true");

        let source = "false";
        let tokens = run(source).expect("false");
        assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "false");
    }

    #[test]
    fn words_longer_than_a_literal_are_identifiers() {
        let cases = [
            "truex",
            "true1",
            "truefalse",
            "falsey",
            "nullable",
            "nulltrue",
            "filterx",
            "filters",
            "filterfilter",
            "TRUE",
            "FALSE",
            "NULL",
            "Filter",
            "t",
            "tr",
            "tru",
            "f",
            "fa",
            "n",
            "nu",
            "nul",
            "fil",
        ];
        for source in cases {
            assert_eq!(
                run(source),
                Ok(vec![tok(JqlTokenKind::Identifier, 0, source.len())]),
                "source {source:?}"
            );
        }

        assert_eq!(
            run("true."),
            Ok(vec![
                tok(JqlTokenKind::Boolean, 0, 4),
                tok(JqlTokenKind::Identifier, 4, 5),
            ])
        );
    }

    #[test]
    fn identifiers_include_dots_and_brackets() {
        for source in [
            ".", ".id", ".name", "foo", "field10", ".filter", "foo[0]", ".[1]", ".1",
        ] {
            assert_eq!(
                run(source),
                Ok(vec![tok(JqlTokenKind::Identifier, 0, source.len())]),
                "source {source:?}"
            );
        }

        assert_eq!(
            run("foo.bar"),
            Ok(vec![
                tok(JqlTokenKind::Identifier, 0, 3),
                tok(JqlTokenKind::Identifier, 3, 7),
            ])
        );
        assert_eq!(
            run(".[1].id"),
            Ok(vec![
                tok(JqlTokenKind::Identifier, 0, 4),
                tok(JqlTokenKind::Identifier, 4, 7),
            ])
        );
    }

    #[test]
    fn numbers_are_digit_runs() {
        for source in ["0", "1", "10", "01", "100"] {
            assert_eq!(
                run(source),
                Ok(vec![tok(JqlTokenKind::Number, 0, source.len())]),
                "number {source:?}"
            );
        }

        assert_eq!(
            run("10x"),
            Ok(vec![
                tok(JqlTokenKind::Number, 0, 2),
                tok(JqlTokenKind::Identifier, 2, 3),
            ])
        );
        assert_eq!(
            run("10("),
            Ok(vec![
                tok(JqlTokenKind::Number, 0, 2),
                tok(JqlTokenKind::LParen, 2, 3),
            ])
        );
        assert_eq!(
            run("1."),
            Ok(vec![
                tok(JqlTokenKind::Number, 0, 1),
                tok(JqlTokenKind::Identifier, 1, 2),
            ])
        );
        assert_eq!(
            run("1.2"),
            Ok(vec![
                tok(JqlTokenKind::Number, 0, 1),
                tok(JqlTokenKind::Identifier, 1, 3),
            ])
        );
    }

    #[test]
    fn strings() {
        let cases = ["\"\"", "\"hi\"", "\"hello world\"", "\"A1\"", "\"a b\""];
        for source in cases {
            assert_eq!(
                run(source),
                Ok(vec![tok(JqlTokenKind::String, 0, source.len())]),
                "string {source:?}"
            );
        }

        assert_eq!(
            run("\"hi\"name"),
            Ok(vec![
                tok(JqlTokenKind::String, 0, 4),
                tok(JqlTokenKind::Identifier, 4, 8),
            ])
        );
        let spaced = "\"a\tb\"";
        assert_eq!(
            run(spaced),
            Ok(vec![tok(JqlTokenKind::String, 0, spaced.len())])
        );
    }

    #[test]
    fn unicode_strings_keep_following_byte_spans() {
        let source = "\"é\"";
        assert_eq!(run(source), Ok(vec![tok(JqlTokenKind::String, 0, 4)]));

        let source = "\"🙂\"";
        assert_eq!(run(source), Ok(vec![tok(JqlTokenKind::String, 0, 6)]));

        let source = "\"é\" true";
        assert_eq!(
            run(source),
            Ok(vec![
                tok(JqlTokenKind::String, 0, 4),
                tok(JqlTokenKind::Boolean, 5, 9),
            ])
        );
    }

    #[test]
    fn whitespace_other_than_space_is_an_error() {
        for source in ["\t", "\n", "\r", "true\tfalse", "true\nfalse"] {
            assert_invalid(source);
        }
    }

    #[test]
    fn unknown_characters_are_errors() {
        for source in [
            "@", "#", "{", "}", "[", "]", ",", "+", "-", "'", "é", "-1", "[0]",
        ] {
            assert_invalid(source);
        }
    }

    #[test]
    fn unterminated_strings_are_errors() {
        for source in ["\"", "\"abc", "\"a\\\"b\""] {
            assert_invalid(source);
        }
    }

    #[test]
    fn invalid_input_does_not_return_a_prefix() {
        for source in ["abc@def", "true @", ".id @", "filter(@"] {
            assert_invalid(source);
        }
    }

    #[test]
    fn readme_pipeline() {
        let source = ".[1] | filter(.id > 10 && .age < 20 && (.money > 10 || .gold >= 1))";
        let tokens = run(source).expect("pipeline");
        assert_eq!(
            kinds(&tokens),
            vec![
                JqlTokenKind::Identifier,
                JqlTokenKind::Pipe,
                JqlTokenKind::Keyword,
                JqlTokenKind::LParen,
                JqlTokenKind::Identifier,
                JqlTokenKind::GreaterOp,
                JqlTokenKind::Number,
                JqlTokenKind::AndLogicalOp,
                JqlTokenKind::Identifier,
                JqlTokenKind::LessOp,
                JqlTokenKind::Number,
                JqlTokenKind::AndLogicalOp,
                JqlTokenKind::LParen,
                JqlTokenKind::Identifier,
                JqlTokenKind::GreaterOp,
                JqlTokenKind::Number,
                JqlTokenKind::OrLogicalOp,
                JqlTokenKind::Identifier,
                JqlTokenKind::GreaterEqualOp,
                JqlTokenKind::Number,
                JqlTokenKind::RParen,
                JqlTokenKind::RParen,
            ]
        );
        let text: Vec<&str> = tokens
            .iter()
            .map(|token| &source[token.span.start..token.span.end])
            .collect();
        assert_eq!(
            text,
            vec![
                ".[1]", "|", "filter", "(", ".id", ">", "10", "&&", ".age", "<", "20", "&&", "(",
                ".money", ">", "10", "||", ".gold", ">=", "1", ")", ")",
            ]
        );
        assert_eq!(tokens[0].span, crate::source::Span { start: 0, end: 4 });
        assert_eq!(tokens[1].span, crate::source::Span { start: 5, end: 6 });
        assert_eq!(tokens[6].span, crate::source::Span { start: 20, end: 22 });
        assert_eq!(
            tokens.last().unwrap().span,
            crate::source::Span { start: 66, end: 67 }
        );
    }
}
