use crate::jql::{
    error::{JqlError, JqlErrorKind},
    token::{JqlToken, JqlTokenKind},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JqlBinaryKind {
    Less,
    Greater,
    Equal,
    NotEqual,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JqlAstNode<'a> {
    LParen,
    RParen,

    // Operator nodes
    Access(&'a str),
    Pipe {
        source: Box<JqlAstNode<'a>>,
        dest: Box<JqlAstNode<'a>>,
    },
    Call {
        name: &'a str,
        args: Vec<JqlAstNode<'a>>,
    },
    Binary {
        kind: JqlBinaryKind,
        left: Box<JqlAstNode<'a>>,
        right: Box<JqlAstNode<'a>>,
    },

    // Operand node
    Number(f32),
    String(&'a str),
    Boolean(bool),
    Null,

    // Marker
    PipeMarker,
    BinaryMarker(JqlBinaryKind),
}

impl<'a> JqlAstNode<'a> {
    pub fn binary(
        kind: JqlToken,
        left: JqlAstNode<'a>,
        right: JqlAstNode<'a>,
    ) -> Result<JqlAstNode<'a>, JqlError> {
        let op_kind = match kind.kind {
            JqlTokenKind::EqualOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::NotEqualOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::LessOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::GreaterOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::LessEqualOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::GreaterEqualOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::AndLogicalOp => Some(JqlBinaryKind::Equal),
            JqlTokenKind::OrLogicalOp => Some(JqlBinaryKind::Equal),
            _ => None,
        };

        if let Some(k) = op_kind {
            return Ok(JqlAstNode::Binary {
                kind: k,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse binary"),
        })
    }

    pub fn call(
        name: JqlToken,
        args: Vec<JqlAstNode<'a>>,
        source: &'a str,
    ) -> Result<JqlAstNode<'a>, JqlError> {
        let err = Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to create call node"),
        });
        if name.kind != JqlTokenKind::Keyword {
            return err;
        }

        Ok(JqlAstNode::Call::<'a> {
            name: source.get(name.span.start..name.span.end).unwrap(),
            args: args,
        })
    }

    pub fn pipe(source: JqlAstNode<'a>, dest: JqlAstNode<'a>) -> Result<JqlAstNode<'a>, JqlError> {
        Ok(JqlAstNode::Pipe {
            source: Box::new(source),
            dest: Box::new(dest),
        })
    }

    pub fn identifier(token: JqlToken, source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
        Ok(JqlAstNode::Access(
            source.get(token.span.start..token.span.end).unwrap(),
        ))
    }

    pub fn number(token: JqlToken, source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
        if token.kind == JqlTokenKind::Number {
            return Ok(JqlAstNode::Number(
                source
                    .get(token.span.start..token.span.end)
                    .unwrap()
                    .parse::<f32>()
                    .unwrap(),
            ));
        }

        Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse number"),
        })
    }

    pub fn string(token: JqlToken, source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
        if token.kind == JqlTokenKind::String {
            return Ok(JqlAstNode::String(
                source
                    .get(token.span.start + 1..token.span.end - 1)
                    .unwrap(),
            ));
        }

        Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse string"),
        })
    }

    pub fn bool(token: JqlToken, source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
        if token.kind == JqlTokenKind::Boolean {
            let value_str = source.get(token.span.start..token.span.end).unwrap();
            if value_str == "true" {
                return Ok(JqlAstNode::Boolean(true));
            }
            if value_str == "false" {
                return Ok(JqlAstNode::Boolean(false));
            }
        }

        Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse boolean"),
        })
    }

    pub fn null(token: JqlToken) -> Result<JqlAstNode<'a>, JqlError> {
        if token.kind == JqlTokenKind::Null {
            return Ok(JqlAstNode::Null);
        }
        Err(JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse boolean"),
        })
    }
}
