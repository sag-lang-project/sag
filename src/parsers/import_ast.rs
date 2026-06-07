use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};

impl Parser {
    pub fn parse_import(&mut self) -> Result<ASTNode, ParseError> {
        self.extract_token(TokenKind::Import);
        let mut symbols = vec![];
        while let Some(token) = self.get_current_token() {
            if token.kind == TokenKind::Comma {
                self.consume_token();
                continue;
            }
            if token.kind == TokenKind::From {
                break;
            }
            match token.kind {
                TokenKind::Identifier(name) => {
                    self.consume_token();
                    symbols.push(name);
                }
                _ => return Err(ParseError::new("Expected identifier", &token)),
            };
        }
        self.extract_token(TokenKind::From);
        let module_name = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Identifier(module_name),
                ..
            }) => module_name.clone(),
            Some(token) => return Err(ParseError::new("Expected module name", &token)),
            None => {
                let (line, column) = match self.get_current_token() {
                    Some(token) => (token.line, token.column),
                    None => (self.line, self.pos),
                };
                return Err(ParseError::new(
                    "Expected module name",
                    &Token {
                        kind: TokenKind::Eof,
                        line,
                        column,
                    },
                ));
            }
        };
        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };
        Ok(ASTNode::Import {
            module_name,
            symbols,
            line,
            column,
        })
    }

    pub fn parse_public(&mut self) -> Result<ASTNode, ParseError> {
        self.extract_token(TokenKind::Pub);
        let (line, column) = match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        };
        Ok(ASTNode::Public {
            node: Box::new(self.parse_expression(0)?),
            line,
            column,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtins;
    use crate::environment::Env;
    use crate::tokenizer::tokenize;
    use crate::value::Value;

    #[test]
    fn test_parse_import() {
        let input = "import foo1, foo2, foo3 from Foo";
        let builtin = register_builtins(&mut Env::new());
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse();
        match ast {
            Ok(ASTNode::Import {
                module_name,
                symbols,
                ..
            }) => {
                assert_eq!(module_name, "Foo");
                assert_eq!(symbols, vec!["foo1", "foo2", "foo3"]);
            }
            _ => panic!("Expected Import"),
        }
    }

    #[test]
    fn test_parse_public() {
        let input = "pub val foo = \"hello\"";
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse();
        match ast {
            Ok(ASTNode::Public { node, .. }) => match *node {
                ASTNode::Assign { name, value, .. } => {
                    assert_eq!(name, "foo");
                    match value.as_ref() {
                        ASTNode::Literal {
                            value: Value::String(v),
                            ..
                        } => assert_eq!(v, "hello"),
                        _ => panic!("Expected String"),
                    }
                }
                _ => panic!("Expected Assignment"),
            },
            _ => panic!("Expected Public"),
        }
    }
}
