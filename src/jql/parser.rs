use crate::jql::{
    ast::{JqlAstNode, JqlBinaryKind},
    error::{JqlError, JqlErrorKind},
    token::{JqlToken, JqlTokenKind},
};

#[derive(Debug, Clone)]
pub enum JqlParserState {
    NewNode,
    InIdentifier,
    InPipe,
    LParen,
    RParen,
    InKeyword,
    StartCall,
    InArgument,
    InCall,
    InString,
    InNumber,
    InNull,
    InBinaryOp(JqlBinaryKind),
    InBoolean,
}

#[derive(Debug, Clone)]
pub struct JqlParser;

const LOGIC_OPS: [JqlTokenKind; 8] = [
    JqlTokenKind::AndLogicalOp,
    JqlTokenKind::OrLogicalOp,
    JqlTokenKind::LessOp,
    JqlTokenKind::GreaterOp,
    JqlTokenKind::LessEqualOp,
    JqlTokenKind::GreaterEqualOp,
    JqlTokenKind::NotEqualOp,
    JqlTokenKind::EqualOp,
];

impl JqlParser {
    fn transition_table(state: &JqlParserState, token: &JqlToken) -> Option<JqlParserState> {
        match state {
            JqlParserState::NewNode => match token.kind {
                JqlTokenKind::Identifier => Some(JqlParserState::InIdentifier),
                JqlTokenKind::Pipe => Some(JqlParserState::InPipe),
                JqlTokenKind::Keyword => Some(JqlParserState::InKeyword),
                JqlTokenKind::LParen => Some(JqlParserState::LParen),
                JqlTokenKind::RParen => Some(JqlParserState::RParen),
                JqlTokenKind::AndLogicalOp
                | JqlTokenKind::OrLogicalOp
                | JqlTokenKind::LessOp
                | JqlTokenKind::LessEqualOp
                | JqlTokenKind::GreaterOp
                | JqlTokenKind::GreaterEqualOp
                | JqlTokenKind::NotEqualOp => Some(JqlParserState::InBinaryOp(
                    Self::convert_token_kind_to_node_kind(&token.kind),
                )),
                JqlTokenKind::Number => Some(JqlParserState::InNumber),
                JqlTokenKind::String => Some(JqlParserState::InString),
                JqlTokenKind::Null => Some(JqlParserState::InNull),
                JqlTokenKind::Boolean => Some(JqlParserState::InBoolean),
                _ => None,
            },
            JqlParserState::InKeyword => match token.kind {
                JqlTokenKind::LParen => Some(JqlParserState::InCall),
                _ => None,
            },
            _ => None,
        }
    }

    // .[1] | filter(.id > 10 && .age < 20 && (.money > 10 || .gold >= 1))
    pub fn parse<'a>(tokens: Vec<JqlToken>, source: &'a str) -> Result<JqlAstNode<'a>, JqlError> {
        println!("{:?}", tokens);

        let mut stack = Vec::<JqlAstNode>::new();
        for token in tokens {
            if token.kind == JqlTokenKind::Pipe {
                stack.push(JqlAstNode::PipeMarker);
                continue;
            }

            if LOGIC_OPS.contains(&token.kind) {
                let bin_node_kind = Self::convert_token_kind_to_node_kind(&token.kind);
                stack.push(JqlAstNode::BinaryMarker(bin_node_kind));
                continue;
            }

            if token.kind == JqlTokenKind::LParen {
                match stack.first() {
                    Some(top) => {
                        stack.push(JqlAstNode::LParen);
                    }
                    _ => {
                        stack.push(JqlAstNode::LParen);
                    }
                }
                continue;
            }

            let node = match token.kind {
                JqlTokenKind::String => JqlAstNode::string(token, source)?,
                JqlTokenKind::Number => JqlAstNode::number(token, source)?,
                JqlTokenKind::Boolean => JqlAstNode::bool(token, source)?,
                JqlTokenKind::Null => JqlAstNode::null(token)?,
                JqlTokenKind::Identifier => JqlAstNode::identifier(token, source)?,
                _ => todo!(),
            };
            // if top is operator marker, let mark operator node
            match stack.first() {
                Some(JqlAstNode::PipeMarker) => {
                    stack.pop();
                    let source_node = stack
                        .pop()
                        .ok_or(JqlError::new(JqlErrorKind::GenericError, String::from("")))?;
                    stack.push(JqlAstNode::pipe(source_node, node)?);
                }
                Some(JqlAstNode::BinaryMarker(bin_kind)) => {
                    let kind = bin_kind.clone();
                    stack.pop();
                    if let Some(left) = stack.pop() {
                        stack.push(JqlAstNode::Binary {
                            kind,
                            left: Box::new(left),
                            right: Box::new(node),
                        })
                    }
                }
                _ => todo!(),
            }
        }

        let err = JqlError {
            kind: JqlErrorKind::GenericError,
            message: String::from("fail to parse"),
        };
        if stack.len() != 1 {
            return Err(err);
        }
        stack.pop().ok_or(err)
    }

    fn convert_token_kind_to_node_kind(kind: &JqlTokenKind) -> JqlBinaryKind {
        match kind {
            JqlTokenKind::AndLogicalOp => JqlBinaryKind::And,
            JqlTokenKind::OrLogicalOp => JqlBinaryKind::Or,
            JqlTokenKind::LessOp => JqlBinaryKind::Less,
            JqlTokenKind::GreaterOp => JqlBinaryKind::GreaterEqual,
            JqlTokenKind::LessEqualOp => JqlBinaryKind::LessEqual,
            JqlTokenKind::GreaterEqualOp => JqlBinaryKind::GreaterEqual,
            JqlTokenKind::NotEqualOp => JqlBinaryKind::NotEqual,
            JqlTokenKind::EqualOp => JqlBinaryKind::Equal,
            _ => todo!(),
        }
    }
}
