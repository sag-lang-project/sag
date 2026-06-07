use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::TokenKind;

impl Parser {
    pub fn parse_option_some(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        self.extract_token(TokenKind::LParen);
        let value = self.parse_expression(0)?;
        self.extract_token(TokenKind::RParen);
        let (line, column) = self.get_line_column();
        Ok(ASTNode::OptionSome {
            value: Box::new(value),
            line,
            column,
        })
    }

    pub fn parse_option_none(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let (line, column) = self.get_line_column();
        Ok(ASTNode::OptionNone { line, column })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtins;
    use crate::environment::Env;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_option_type_other_type_error() {
        let input = r#"
        val mut x:Option<number> = None
        x = Some("hello")
        "#
        .to_string();
        let mut env = Env::new();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        match ast.unwrap_err() {
            ParseError { message, .. } => {
                assert_eq!(message, "type mismatch");
            }
        }
    }
    #[test]
    fn test_other_type_reassign_some_error() {
        let input = r#"
        val mut x = 1
        x = Some("hello")
        "#
        .to_string();
        let mut env = Env::new();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        match ast.unwrap_err() {
            ParseError { message, .. } => {
                assert_eq!(message, "type mismatch");
            }
        }
    }
    #[test]
    fn test_other_type_reassign_none_error() {
        let input = r#"
        val mut x = 1
        x = None
        "#
        .to_string();
        let mut env = Env::new();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        match ast.unwrap_err() {
            ParseError { message, .. } => {
                assert_eq!(message, "type mismatch");
            }
        }
    }
}
