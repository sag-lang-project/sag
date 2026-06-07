use crate::ast::ASTNode;
use crate::environment::ValueType;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_if(&mut self) -> Result<ASTNode, ParseError> {
        // if式か文か
        let mut is_statement = false;
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::If,
                ..
            }) => self.consume_token(),
            _ => {
                let current_token = self.get_current_token().unwrap();
                return Err(ParseError::new(
                    "unexpected token missing if",
                    &current_token,
                ));
            }
        };
        let condition = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::LParen,
                ..
            }) => {
                self.consume_token(); // Consume the left parenthesis
                let expr = self.parse_expression(0)?;
                // Check for and consume the right parenthesis
                match self.get_current_token() {
                    Some(Token {
                        kind: TokenKind::RParen,
                        ..
                    }) => {
                        self.consume_token();
                        expr
                    }
                    _ => {
                        let current_token = self.get_current_token().unwrap_or(Token {
                            kind: TokenKind::Eof,
                            line: self.line,
                            column: self.pos,
                        });
                        return Err(ParseError::new(
                            "unexpected token missing )",
                            &current_token,
                        ));
                    }
                }
            }
            _ => {
                let current_token = self.get_current_token().unwrap_or(Token {
                    kind: TokenKind::Eof,
                    line: self.line,
                    column: self.pos,
                });
                return Err(ParseError::new(
                    "unexpected token missing (",
                    &current_token,
                ));
            }
        };
        let then = self.parse_expression(0)?;
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Eof,
                ..
            }) => {
                self.pos = 0;
                self.line += 1;
            }
            _ => {}
        };
        let else_ = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Else,
                ..
            }) => {
                self.consume_token();
                match self.get_current_token() {
                    Some(Token {
                        kind: TokenKind::If,
                        ..
                    }) => Some(Box::new(self.parse_if()?)),
                    _ => Some(Box::new(self.parse_expression(0)?)),
                }
            }
            _ => None,
        };

        let value_type = {
            let mut then_type = None;
            let mut else_type = None;

            match then {
                ASTNode::Return {
                    expr: ref value, ..
                } => {
                    then_type = Some(self.infer_type(&value));
                }
                ASTNode::Block {
                    nodes: ref statements,
                    ..
                } => {
                    for statement in statements {
                        if let ASTNode::Return { expr: value, .. } = statement {
                            is_statement = true;
                            then_type = Some(self.infer_type(&value));
                        }
                    }
                    if then_type.is_none() {
                        let last = statements.last();
                        match last {
                            Some(ast_node) => {
                                then_type = Some(self.infer_type(ast_node));
                            }
                            _ => then_type = None,
                        }
                    }
                }
                _ => {}
            }

            if let Some(else_node) = &else_ {
                match &**else_node {
                    ASTNode::Return { expr: value, .. } => {
                        is_statement = true;
                        else_type = Some(self.infer_type(&value));
                    }
                    ASTNode::Block {
                        nodes: statements, ..
                    } => {
                        for statement in statements {
                            if let ASTNode::Return { expr: value, .. } = statement {
                                else_type = Some(self.infer_type(&value));
                            }
                        }
                        if then_type.is_none() {
                            let last = statements.last();
                            match last {
                                Some(ast_node) => {
                                    else_type = Some(self.infer_type(ast_node));
                                }
                                _ => else_type = None,
                            }
                        }
                    }
                    _ => {}
                }
            }

            match (then_type, else_type) {
                (Some(t), Some(e)) if t == e => t,
                (Some(t), None) => t,
                (None, Some(e)) => e,
                (None, None) => Ok(ValueType::Void),
                _ => Err("Type mismatch in if statement".to_string()),
            }
        };
        if value_type.is_err() {
            if let Some(token) = self.get_current_token() {
                return Err(ParseError::new(&value_type.err().unwrap(), &token));
            }
        }

        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };

        Ok(ASTNode::If {
            condition: Box::new(condition),
            is_statement,
            then: Box::new(then),
            else_,
            value_type: value_type.unwrap(),
            line,
            column,
        })
    }
}
