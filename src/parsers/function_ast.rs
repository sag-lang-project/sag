use crate::ast::ASTNode;
use crate::environment::{EnvVariableType, ValueType};
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_function(&mut self) -> Result<ASTNode, ParseError> {
        self.pos += 1;
        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };
        let name = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) => name,
            _ => Err(ParseError::new(
                "Expected function name",
                &self.get_current_token().unwrap(),
            ))?,
        };
        let function_scope = self.get_current_scope();
        self.enter_scope(name.to_string());
        self.pos += 1;
        self.extract_token(TokenKind::LParen);

        let arguments = self.parse_function_arguments()?;
        let return_type = self.parse_return_type();
        self.register_functions(function_scope, &name, &arguments, &return_type);
        let body = self.parse_block()?;

        self.leave_scope();

        let mut include_return = false;
        match body.clone() {
            ASTNode::Block {
                nodes: statements, ..
            } => {
                for statement in statements {
                    if let ASTNode::Return { expr: value, .. } = statement {
                        include_return = true;
                        if let Ok(return_value_type) = self.infer_type(&value.clone()) {
                            if return_value_type != return_type {
                                return Err(ParseError::new(
                                    format!("Return type mismatch Expected type: {:?}, Actual type: {:?}", return_type, return_value_type).as_str(),
                                    &self.get_current_token().unwrap()
                                    )
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        };

        if !include_return && return_type != ValueType::Void {
            let token = self.get_current_token().unwrap();
            Err(ParseError::new("Expected return statement", &token))?;
        }

        Ok(ASTNode::Function {
            name,
            arguments,
            body: Box::new(body),
            return_type,
            line,
            column,
        })
    }

    pub fn parse_function_call_arguments_paren(&mut self) -> Result<ASTNode, ParseError> {
        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::LParen,
                ..
            }) => self.consume_token(),
            _ => None,
        };
        let mut arguments = vec![];
        while let Some(token) = self.get_current_token() {
            if token.kind == TokenKind::Comma {
                self.pos += 1;
                continue;
            }
            if token.kind == TokenKind::RParen {
                self.pos += 1;
                break;
            }
            if token.kind == TokenKind::Eof {
                self.pos = 0;
                self.line += 1;
                continue;
            }
            let value = self.parse_expression(0)?;
            arguments.push(value);
        }
        Ok(ASTNode::FunctionCallArgs {
            args: arguments,
            line,
            column,
        })
    }

    pub fn parse_function_arguments(&mut self) -> Result<Vec<ASTNode>, ParseError> {
        let scope = self.get_current_scope();
        let mut arguments = Vec::new();
        while let Some(token) = self.get_current_token() {
            if token.kind == TokenKind::RParen {
                break;
            }
            if let TokenKind::Identifier(name) = self.consume_token().unwrap().kind {
                let mut variable_name = name.clone();
                let current_token = self.get_current_token();
                let arg_type = if current_token.is_none() {
                    self.extract_token(TokenKind::Colon);
                    match self.consume_token() {
                        Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) => self.string_to_value_type(type_name),
                        _ => Err(ParseError::new(
                            "Expected type for argument",
                            &self.get_current_token().unwrap(),
                        ))?,
                    }
                } else {
                    let current_token_kind = current_token.unwrap().kind.clone();
                    if name == "self"
                        && (current_token_kind == TokenKind::Comma
                            || current_token_kind == TokenKind::RParen)
                    {
                        ValueType::SelfType
                    } else if name == "mut"
                        && current_token_kind == TokenKind::Identifier("self".to_string())
                    {
                        self.consume_token();
                        let current_token_kind = self.get_current_token().unwrap().kind.clone();
                        if current_token_kind == TokenKind::Comma
                            || current_token_kind == TokenKind::RParen
                        {
                            variable_name = "self".to_string();
                            ValueType::MutSelfType
                        } else {
                            Err(ParseError::new(
                                "Expected type for argument",
                                &self.get_current_token().unwrap(),
                            ))?
                        }
                    } else {
                        self.extract_token(TokenKind::Colon);
                        match self.consume_token() {
                            Some(Token {
                                kind: TokenKind::Identifier(type_name),
                                ..
                            }) => self.string_to_value_type(type_name),
                            _ => Err(ParseError::new(
                                "Expected type for argument",
                                &self.get_current_token().unwrap(),
                            ))?,
                        }
                    }
                };
                self.register_variables(
                    scope.to_string(),
                    &variable_name,
                    &arg_type,
                    &EnvVariableType::Immutable,
                );
                let (line, column) = match self.get_current_token() {
                    Some(token) => (token.line, token.column),
                    None => (self.line, self.pos),
                };
                arguments.push(ASTNode::Variable {
                    name: variable_name,
                    value_type: Some(arg_type),
                    line,
                    column,
                });
            }
            match self.get_current_token() {
                Some(Token {
                    kind: TokenKind::Comma,
                    ..
                }) => {
                    self.consume_token();
                }
                _ => {}
            };
        }
        self.extract_token(TokenKind::RParen);
        Ok(arguments)
    }

    pub fn parse_function_call_front(
        &mut self,
        name: String,
        arguments: ASTNode,
    ) -> Result<ASTNode, ParseError> {
        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };
        Ok(ASTNode::FunctionCall {
            name,
            arguments: Box::new(arguments),
            line,
            column,
        })
    }

    pub fn parse_function_call(&mut self, left: ASTNode) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let name = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) => name,
            _ => Err(ParseError::new(
                "Expected function name",
                &self.get_current_token().unwrap(),
            ))?,
        };

        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };

        let arguments = ASTNode::FunctionCallArgs {
            args: match left {
                ASTNode::FunctionCallArgs {
                    args: arguments, ..
                } => arguments,
                _ => vec![left],
            },
            line,
            column,
        };

        self.consume_token();

        Ok(ASTNode::FunctionCall {
            name,
            arguments: Box::new(arguments),
            line,
            column,
        })
    }
}
