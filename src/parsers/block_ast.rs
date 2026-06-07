use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_block(&mut self) -> Result<ASTNode, ParseError> {
        let mut statements = Vec::new();
        match self.get_current_token() {
            Some(Token {
                kind: TokenKind::LBrace,
                ..
            }) => {
                self.consume_token();
            }
            _ => {
                let (line, column) = self.get_line_column();
                return Err(ParseError {
                    message: "Expected '{' at the start of a block".to_string(),
                    line,
                    column,
                });
            }
        }

        loop {
            let token = self.get_current_token();
            match token {
                Some(Token {
                    kind: TokenKind::RBrace,
                    ..
                }) => {
                    self.consume_token();
                    break;
                }
                Some(Token {
                    kind: TokenKind::Eof,
                    ..
                }) => {
                    self.pos = 0;
                    self.line += 1;
                    continue;
                }
                None => {
                    if self.line >= self.tokens.len() {
                        let (line, column) = self.get_line_column();
                        return Err(ParseError {
                            message: "Unexpected end of file, expected '}'".to_string(),
                            line,
                            column,
                        });
                    }
                    self.pos = 0;
                    self.line += 1;
                    continue;
                }
                _ => {
                    let statement = self.parse_expression(0)?;
                    statements.push(statement);
                }
            }
        }

        let (line, column) = self.get_line_column();
        Ok(ASTNode::Block {
            nodes: statements,
            line,
            column,
        })
    }
}
