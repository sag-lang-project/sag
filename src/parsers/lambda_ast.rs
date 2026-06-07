use crate::ast::ASTNode;
use crate::environment::EnvVariableType;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_lambda(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let mut arguments = vec![];

        self.enter_scope("lambda".to_string());
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Pipe,
                ..
            }) => {
                self.consume_token();
                while let Some(token) = self.get_current_token() {
                    if token.kind == TokenKind::Pipe {
                        self.consume_token();
                        break;
                    }
                    match self.get_current_token() {
                        Some(Token {
                            kind: TokenKind::Comma,
                            ..
                        }) => {
                            self.consume_token();
                            continue;
                        }
                        _ => {}
                    };
                    if let TokenKind::Identifier(argument) = token.kind {
                        self.consume_token();
                        self.extract_token(TokenKind::Colon);
                        let value_type = if let Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) = self.get_current_token()
                        {
                            Some(self.string_to_value_type(type_name))
                        } else {
                            None
                        };

                        let (line, column) = self.get_line_column();
                        arguments.push(ASTNode::Variable {
                            name: argument.clone(),
                            value_type: value_type.clone(),
                            line,
                            column,
                        });
                        self.register_variables(
                            "lambda".to_string(),
                            &argument,
                            &value_type.unwrap(),
                            &EnvVariableType::Immutable,
                        );
                        self.consume_token();
                        continue;
                    }
                }
            }
            Some(Token {
                kind: TokenKind::Identifier(argument),
                ..
            }) => {
                self.consume_token();
                self.extract_token(TokenKind::Colon);
                let value_type = if let Some(Token {
                    kind: TokenKind::Identifier(type_name),
                    ..
                }) = self.get_current_token()
                {
                    Some(self.string_to_value_type(type_name))
                } else {
                    None
                };
                let (line, column) = self.get_line_column();
                arguments.push(ASTNode::Variable {
                    name: argument.clone(),
                    value_type,
                    line,
                    column,
                });
                self.consume_token();
            }
            _ => {}
        };

        self.extract_token(TokenKind::RRocket);

        let result = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::LBrace,
                ..
            }) => {
                let statement = self.parse_block()?;
                let (line, column) = self.get_line_column();
                ASTNode::Lambda {
                    arguments,
                    body: Box::new(statement),
                    line,
                    column,
                }
            }
            _ => {
                let statement = self.parse_expression(0)?;
                let (line, column) = self.get_line_column();
                ASTNode::Lambda {
                    arguments,
                    body: Box::new(statement),
                    line,
                    column,
                }
            }
        };
        self.leave_scope();
        Ok(result)
    }

    pub fn parse_lambda_call(&mut self, left: ASTNode) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let lambda = self.parse_lambda()?;
        let arguments = match left {
            ASTNode::FunctionCallArgs {
                args: arguments, ..
            } => arguments,
            _ => vec![left],
        };
        let (line, column) = self.get_line_column();
        Ok(ASTNode::LambdaCall {
            lambda: Box::new(lambda),
            arguments,
            line,
            column,
        })
    }

    pub fn is_lambda_call(&mut self) -> bool {
        self.pos += 1;
        let next_token = self.get_current_token();
        self.pos -= 1;
        match next_token {
            Some(Token {
                kind: TokenKind::BackSlash,
                ..
            }) => true,
            _ => false,
        }
    }
}
