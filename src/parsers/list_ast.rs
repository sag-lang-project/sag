use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::TokenKind;
use crate::value::Value;

impl Parser {
    pub fn parse_list(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let mut list = vec![];
        while let Some(token) = self.get_current_token() {
            if token.kind == TokenKind::RBrancket {
                self.consume_token();
                break;
            }
            if token.kind == TokenKind::Comma {
                self.consume_token();
                continue;
            }
            let value = match token.kind {
                TokenKind::Number(value) => Value::Number(value),
                TokenKind::String(value) => Value::String(value),
                _ => panic!("unexpected token: {:?}", token),
            };
            list.push(ASTNode::Literal {
                value,
                line: token.line,
                column: token.column,
            });
            self.consume_token();
        }
        let (line, column) = self.get_line_column();
        Ok(ASTNode::Literal {
            value: Value::List(
                list.iter()
                    .map(|x| match x {
                        ASTNode::Literal { value, .. } => value.clone(),
                        _ => panic!("unexpected node"),
                    })
                    .collect(),
            ),
            line,
            column,
        })
    }
}
