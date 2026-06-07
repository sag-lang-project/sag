use crate::ast::ASTNode;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::TokenKind;
use crate::value::Value;
use std::collections::HashMap;

impl Parser {
    pub fn parse_dict(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let mut dict = HashMap::new();
        let mut key = None;
        while let Some(token) = self.get_current_token() {
            if token.kind == TokenKind::Colon {
                self.pos += 1;
                let next_token = self.get_current_token();
                if next_token.unwrap().kind == TokenKind::RBrace {
                    self.consume_token();
                    break;
                }
                self.pos -= 1;
            }
            if token.kind == TokenKind::RRocket {
                self.consume_token();
                if key.is_none() {
                    panic!("Expected key-value pair in dictionary");
                }
                let value = self.get_current_token().map(|t| match t.kind {
                    TokenKind::Number(value) => Value::Number(value),
                    TokenKind::String(value) => Value::String(value),
                    _ => panic!("unexpected token: {:?}", t),
                });
                self.consume_token();
                if let Some(Value::String(k)) = key {
                    dict.insert(k, value.unwrap());
                } else {
                    panic!("Expected string key in dictionary");
                }
                key = None;
                continue;
            }
            if token.kind == TokenKind::Comma {
                self.consume_token();
                continue;
            }
            key = match token.kind {
                TokenKind::String(value) => Some(Value::String(value)),
                _ => panic!("unexpected token: {:?}", token),
            };
            self.consume_token();
        }
        let (line, column) = self.get_line_column();
        Ok(ASTNode::Literal {
            value: Value::Dict(dict),
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
    use fraction::Fraction;

    #[test]
    fn test_parse_empty_dict() {
        let input = r#"val d = {::}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_parse_dict_with_number_value() {
        let input = r#"val d = {: "key" => 42 :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 1);
                    assert_eq!(dict.get("key"), Some(&Value::Number(Fraction::from(42))));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_with_string_value() {
        let input = r#"val d = {: "name" => "Alice" :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 1);
                    assert_eq!(dict.get("name"), Some(&Value::String("Alice".to_string())));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_with_multiple_entries() {
        let input = r#"val d = {: "a" => 5, "b" => "hello", "c" => 10 :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 3);
                    assert_eq!(dict.get("a"), Some(&Value::Number(Fraction::from(5))));
                    assert_eq!(dict.get("b"), Some(&Value::String("hello".to_string())));
                    assert_eq!(dict.get("c"), Some(&Value::Number(Fraction::from(10))));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_access() {
        let input = r#"
val v = {: "a" => 5, "b" => 2 :}
v["a"]
"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());
    }

    #[test]
    fn test_parse_dict_with_decimal_numbers() {
        let input = r#"val d = {: "pi" => 3.14, "e" => 2.71 :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 2);
                    assert_eq!(dict.get("pi"), Some(&Value::Number(Fraction::from(3.14))));
                    assert_eq!(dict.get("e"), Some(&Value::Number(Fraction::from(2.71))));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_with_single_entry() {
        let input = r#"val d = {: "single" => 1 :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 1);
                    assert_eq!(dict.get("single"), Some(&Value::Number(Fraction::from(1))));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_with_mixed_values() {
        let input = r#"val d = {: "number" => 42, "text" => "hello", "decimal" => 3.14 :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 3);
                    assert_eq!(dict.get("number"), Some(&Value::Number(Fraction::from(42))));
                    assert_eq!(dict.get("text"), Some(&Value::String("hello".to_string())));
                    assert_eq!(
                        dict.get("decimal"),
                        Some(&Value::Number(Fraction::from(3.14)))
                    );
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }

    #[test]
    fn test_parse_dict_with_spaces() {
        let input = r#"val d = {: "key1" => 1 , "key2" => "value" :}"#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        assert!(ast.is_ok());

        // ASTの内容を検証
        let ast_nodes = ast.unwrap();
        if let Some(assign_node) = ast_nodes.first() {
            if let ASTNode::Assign { value, .. } = assign_node {
                if let ASTNode::Literal {
                    value: Value::Dict(dict),
                    ..
                } = value.as_ref()
                {
                    assert_eq!(dict.len(), 2);
                    assert_eq!(dict.get("key1"), Some(&Value::Number(Fraction::from(1))));
                    assert_eq!(dict.get("key2"), Some(&Value::String("value".to_string())));
                } else {
                    panic!("Expected Dict literal");
                }
            } else {
                panic!("Expected Assign node");
            }
        } else {
            panic!("Expected at least one AST node");
        }
    }
}
