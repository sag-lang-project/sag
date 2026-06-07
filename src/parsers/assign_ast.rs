use crate::ast::ASTNode;
use crate::environment::{EnvVariableType, ValueType};
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    fn get_result_value_type(&mut self) -> Result<ValueType, ParseError> {
        match self.consume_token() {
            Some(token) => match token.kind {
                TokenKind::Identifier(value_type) => Ok(self.string_to_value_type(value_type)),
                TokenKind::Option => {
                    self.extract_token(TokenKind::Lt);
                    let value_type = match self.consume_token() {
                        Some(token) => match token.kind {
                            TokenKind::Identifier(value_type) => {
                                self.string_to_value_type(value_type)
                            }
                            TokenKind::Option => {
                                self.extract_token(TokenKind::Lt);
                                let result = self.get_result_value_type()?;
                                self.extract_token(TokenKind::Gt);
                                ValueType::OptionType(Box::new(result))
                            }
                            TokenKind::Result => {
                                self.extract_token(TokenKind::Lt);
                                let success_value_type = self.get_result_value_type()?;
                                self.extract_token(TokenKind::Comma);
                                let failure_value_type = self.get_result_value_type()?;
                                self.extract_token(TokenKind::Gt);
                                ValueType::ResultType {
                                    success: Box::new(success_value_type),
                                    failure: Box::new(failure_value_type),
                                }
                            }
                            TokenKind::List => {
                                self.extract_token(TokenKind::Lt);
                                let element_type = self.get_result_value_type()?;
                                self.extract_token(TokenKind::Gt);
                                ValueType::List(Box::new(element_type))
                            }
                            _ => return Err(ParseError::new("unexpected token", &token)),
                        },
                        _ => return Err(ParseError::new("unexpected token", &token)),
                    };
                    self.extract_token(TokenKind::Gt);
                    Ok(ValueType::OptionType(Box::new(value_type)))
                }
                TokenKind::Result => {
                    self.extract_token(TokenKind::Lt);
                    let success_value_type = self.get_result_value_type()?;
                    self.extract_token(TokenKind::Comma);
                    let failure_value_type = self.get_result_value_type()?;
                    self.extract_token(TokenKind::Gt);
                    Ok(ValueType::ResultType {
                        success: Box::new(success_value_type),
                        failure: Box::new(failure_value_type),
                    })
                }
                TokenKind::List => {
                    self.extract_token(TokenKind::Lt);
                    let element_type = self.get_result_value_type()?;
                    self.extract_token(TokenKind::Gt);
                    Ok(ValueType::List(Box::new(element_type)))
                }
                _ => Err(ParseError::new("unexpected token", &token)),
            },
            _ => Err(ParseError::new(
                "unexpected token",
                &Token {
                    kind: TokenKind::Eof,
                    line: 0,
                    column: 0,
                },
            )),
        }
    }

    pub fn parse_assign(&mut self) -> Result<ASTNode, ParseError> {
        let scope = self.get_current_scope();
        let mutable_or_immutable = self.consume_token().unwrap();
        let name = match self.consume_token() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) => name,
            _ => {
                let current_token = self.get_current_token().unwrap();
                return Err(ParseError::new(
                    "unexpected token missing variable name",
                    &current_token,
                ));
            }
        };
        match self.consume_token() {
            Some(Token {
                kind: TokenKind::Equal,
                ..
            }) => {
                let value = self.parse_expression(0)?;
                let value_type = match self.infer_type(&value) {
                    Ok(value_type) => value_type,
                    Err(e) => panic!("{}", e),
                };
                let variable_type = if mutable_or_immutable.kind == TokenKind::Mutable {
                    EnvVariableType::Mutable
                } else {
                    EnvVariableType::Immutable
                };

                self.register_variables(scope.clone(), &name, &value_type, &variable_type);
                Ok(ASTNode::Assign {
                    name,
                    value: Box::new(value),
                    variable_type,
                    value_type,
                    is_new: true,
                    line: mutable_or_immutable.line,
                    column: mutable_or_immutable.column,
                })
            }
            Some(Token {
                kind: TokenKind::Colon,
                ..
            }) => {
                let value_type = match self.consume_token() {
                    Some(token) => match token.kind {
                        TokenKind::Identifier(value_type) => match value_type.as_str() {
                            "number" => ValueType::Number,
                            "str" => ValueType::String,
                            "bool" => ValueType::Bool,
                            "void" => ValueType::Void,
                            _ => self.string_to_value_type(value_type),
                        },
                        TokenKind::Option => {
                            self.extract_token(TokenKind::Lt);
                            let value_type = match self.consume_token() {
                                Some(token) => match token.kind {
                                    TokenKind::Identifier(value_type) => {
                                        self.string_to_value_type(value_type)
                                    }
                                    _ => return Err(ParseError::new("unexpected token", &token)),
                                },
                                _ => return Err(ParseError::new("unexpected token", &token)),
                            };
                            self.extract_token(TokenKind::Gt);
                            ValueType::OptionType(Box::new(value_type))
                        }
                        TokenKind::Result => {
                            self.extract_token(TokenKind::Lt);
                            let success_value_type = self.get_result_value_type()?;
                            self.extract_token(TokenKind::Comma);
                            let failure_value_type = self.get_result_value_type()?;
                            self.extract_token(TokenKind::Gt);
                            ValueType::ResultType {
                                success: Box::new(success_value_type),
                                failure: Box::new(failure_value_type),
                            }
                        }
                        _ => return Err(ParseError::new("unexpected token", &token)),
                    },
                    _ => panic!("missing token"),
                };
                let token = self.consume_token();
                match token {
                    Some(Token {
                        kind: TokenKind::Equal,
                        ..
                    }) => {
                        let value = self.parse_expression(0)?;
                        let variable_type = if mutable_or_immutable.kind == TokenKind::Mutable {
                            EnvVariableType::Mutable
                        } else {
                            EnvVariableType::Immutable
                        };
                        match value_type.clone() {
                            ValueType::ResultType { success, failure } => match value {
                                ASTNode::ResultSuccess { ref value, .. } => {
                                    if *success.as_ref() != self.infer_type(&value).unwrap() {
                                        return Err(ParseError::new(
                                            "type mismatch",
                                            &token.unwrap(),
                                        ));
                                    }
                                }
                                ASTNode::ResultFailure { ref value, .. } => {
                                    if *failure.as_ref() != self.infer_type(&value).unwrap() {
                                        return Err(ParseError::new(
                                            "type mismatch",
                                            &token.unwrap(),
                                        ));
                                    }
                                }
                                _ => return Err(ParseError::new("type mismatch", &token.unwrap())),
                            },
                            ValueType::OptionType(ref value_type) => match value {
                                ASTNode::OptionSome { ref value, .. } => {
                                    if *value_type.as_ref() != self.infer_type(&value).unwrap() {
                                        return Err(ParseError::new(
                                            "type mismatch",
                                            &token.unwrap(),
                                        ));
                                    }
                                }
                                ASTNode::OptionNone { .. } => {}
                                _ => return Err(ParseError::new("type mismatch", &token.unwrap())),
                            },
                            _ => {
                                if value_type != self.infer_type(&value).unwrap() {
                                    return Err(ParseError::new("type mismatch", &token.unwrap()));
                                }
                            }
                        }
                        self.register_variables(scope, &name, &value_type, &variable_type);
                        Ok(ASTNode::Assign {
                            name,
                            value: Box::new(value),
                            variable_type,
                            value_type,
                            is_new: true,
                            line: mutable_or_immutable.line,
                            column: mutable_or_immutable.column,
                        })
                    }
                    _ => panic!("No valid statement found on the right-hand side"),
                }
            }
            _ => panic!("unexpected token"),
        }
    }
}
