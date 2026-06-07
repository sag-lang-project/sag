use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::value::Value;

impl Parser {
    pub fn parse_literal(&mut self, value: Value) -> Result<ASTNode, ParseError> {
        self.pos += 1;
        let (line, column) = self.get_line_column();
        Ok(ASTNode::Literal {
            value,
            line,
            column,
        })
    }
}
