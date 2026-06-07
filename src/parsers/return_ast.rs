use crate::ast::ASTNode;
use crate::environment::ValueType;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_return(&mut self) -> Result<ASTNode, ParseError> {
        self.pos += 1;
        let value = self.parse_expression(0)?;
        let (line, column) = self.get_line_column();
        Ok(ASTNode::Return {
            expr: Box::new(value),
            line,
            column,
        })
    }

    pub fn parse_return_type(&mut self) -> ValueType {
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Colon,
                ..
            }) => {
                self.consume_token();
                if let Some(Token {
                    kind: TokenKind::Identifier(type_name),
                    ..
                }) = self.get_current_token()
                {
                    self.consume_token();
                    return self.string_to_value_type(type_name);
                }
                if let Some(Token {
                    kind: TokenKind::Option,
                    ..
                }) = self.get_current_token()
                {
                    self.consume_token();
                    self.extract_token(TokenKind::Lt);
                    let some = match self.get_current_token() {
                        Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) => {
                            self.consume_token();
                            self.string_to_value_type(type_name)
                        }
                        _ => ValueType::Void,
                    };
                    self.extract_token(TokenKind::Gt);
                    return ValueType::OptionType(Box::new(some));
                }
                if let Some(Token {
                    kind: TokenKind::Result,
                    ..
                }) = self.get_current_token()
                {
                    self.consume_token();
                    self.extract_token(TokenKind::Lt);
                    let success = match self.get_current_token() {
                        Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) => {
                            self.consume_token();
                            self.string_to_value_type(type_name)
                        }
                        _ => ValueType::Void,
                    };
                    self.extract_token(TokenKind::Comma);
                    let failure = match self.get_current_token() {
                        Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) => {
                            self.consume_token();
                            self.string_to_value_type(type_name)
                        }
                        _ => ValueType::Void,
                    };
                    self.consume_token();
                    return ValueType::ResultType {
                        success: Box::new(success),
                        failure: Box::new(failure),
                    };
                }
                if let Some(Token {
                    kind: TokenKind::List,
                    ..
                }) = self.get_current_token()
                {
                    self.consume_token();
                    self.extract_token(TokenKind::Lt);
                    let element_type = match self.get_current_token() {
                        Some(Token {
                            kind: TokenKind::Identifier(type_name),
                            ..
                        }) => {
                            self.consume_token();
                            self.string_to_value_type(type_name)
                        }
                        _ => ValueType::Void,
                    };
                    self.extract_token(TokenKind::Gt);
                    return ValueType::List(Box::new(element_type));
                }
            }
            _ => {}
        };
        ValueType::Void
    }
}
