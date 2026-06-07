use crate::ast::ASTNode;
use crate::environment::{EnvVariableType, ValueType};
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::TokenKind;

impl Parser {
    pub fn parse_match(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let expression = self.parse_expression(0)?;
        self.extract_token(TokenKind::LBrace);
        let mut cases = vec![];
        let case_pattern_type: Result<ValueType, String> = self.infer_type(&expression);
        let mut case_body_type: Option<ValueType> = None;
        let mut under_score_pattern_count = 0;

        let mut count = 0;

        while self.get_current_token().is_some()
            && self.get_current_token().unwrap().kind != TokenKind::RBrace
        {
            if self.get_current_token().unwrap().kind == TokenKind::Eof {
                self.pos = 0;
                self.line += 1;
                continue;
            }
            let pattern = self.parse_expression(0)?;
            self.enter_scope(format!("match-{:?}", count).to_string());
            count += 1;
            match pattern {
                ASTNode::OptionSome { ref value, .. } => {
                    match *value.clone() {
                        ASTNode::Variable { name, .. } => {
                            let expression_type = self.infer_type(&expression);
                            match expression_type {
                                Ok(ValueType::OptionType(some)) => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &some,
                                        &EnvVariableType::Immutable,
                                    );
                                }
                                _ => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &expression_type.unwrap(),
                                        &EnvVariableType::Immutable,
                                    );
                                }
                            }
                        }
                        _ => {}
                    };
                }
                ASTNode::ResultSuccess { ref value, .. } => {
                    match *value.clone() {
                        ASTNode::Variable { name, .. } => {
                            let expression_type = self.infer_type(&expression);
                            match expression_type {
                                Ok(ValueType::ResultType { success, .. }) => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &success,
                                        &EnvVariableType::Immutable,
                                    );
                                }
                                _ => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &expression_type.unwrap(),
                                        &EnvVariableType::Immutable,
                                    );
                                }
                            }
                        }
                        _ => {}
                    };
                }
                ASTNode::ResultFailure { ref value, .. } => {
                    match *value.clone() {
                        ASTNode::Variable { name, .. } => {
                            let expression_type = self.infer_type(&expression);
                            match expression_type {
                                Ok(ValueType::ResultType { failure, .. }) => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &failure,
                                        &EnvVariableType::Immutable,
                                    );
                                    println!("variable!!: {:?}", self.variables);
                                }
                                _ => {
                                    self.register_variables(
                                        self.get_current_scope().clone(),
                                        &name,
                                        &expression_type.unwrap(),
                                        &EnvVariableType::Immutable,
                                    );
                                }
                            }
                        }
                        _ => {}
                    };
                }
                _ => {}
            };
            self.extract_token(TokenKind::RRocket);
            let body = self.parse_block()?;
            cases.push((pattern.clone(), body.clone()));
            let is_underscore = match pattern {
                ASTNode::Variable { ref name, .. } => name == "_",
                _ => false,
            };
            if is_underscore {
                under_score_pattern_count += 1;
                if under_score_pattern_count >= 2 {
                    return Err(ParseError::new(
                        "too many wild card pattern _",
                        &self.get_current_token().unwrap(),
                    ));
                }
            }
            if !is_underscore && self.infer_type(&pattern).is_err() {
                return Err(ParseError::new(
                    "Unsupported pattern",
                    &self.get_current_token().unwrap(),
                ));
            }
            if case_body_type.is_none() {
                case_body_type = self.infer_type(&body).ok();
                if case_pattern_type.is_err() {
                    return Err(ParseError::new(
                        "Pattern type mismatch",
                        &self.get_current_token().unwrap(),
                    ));
                }
            } else if case_body_type != self.infer_type(&body).ok() {
                return Err(ParseError::new(
                    "Pattern type mismatch",
                    &self.get_current_token().unwrap(),
                ));
            }
            self.leave_scope();
        }

        self.extract_token(TokenKind::RBrace);
        let (line, column) = self.get_line_column();
        Ok(ASTNode::Match {
            expression: Box::new(expression),
            cases,
            line,
            column,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::ASTNode;
    use crate::parsers::Parser;
    use crate::token::{Token, TokenKind};
    use crate::value::Value;
    use fraction::Fraction;
    use std::collections::HashMap;

    #[test]
    fn test_parse_match() {
        let mut parser = Parser::new(
            vec![
                Token {
                    kind: TokenKind::Match,
                    line: 1,
                    column: 1,
                },
                Token {
                    kind: TokenKind::LParen,
                    line: 1,
                    column: 7,
                },
                Token {
                    kind: TokenKind::Number(Fraction::from(0)),
                    line: 1,
                    column: 8,
                },
                Token {
                    kind: TokenKind::RParen,
                    line: 1,
                    column: 9,
                },
                Token {
                    kind: TokenKind::LBrace,
                    line: 1,
                    column: 11,
                },
                Token {
                    kind: TokenKind::Number(Fraction::from(1)),
                    line: 1,
                    column: 13,
                },
                Token {
                    kind: TokenKind::RRocket,
                    line: 1,
                    column: 15,
                },
                Token {
                    kind: TokenKind::LBrace,
                    line: 1,
                    column: 17,
                },
                Token {
                    kind: TokenKind::Number(Fraction::from(2)),
                    line: 1,
                    column: 18,
                },
                Token {
                    kind: TokenKind::RBrace,
                    line: 1,
                    column: 20,
                },
                Token {
                    kind: TokenKind::RBrace,
                    line: 1,
                    column: 20,
                },
            ],
            HashMap::new(),
        );
        let result = parser.parse();
        assert_eq!(result.is_ok(), true);
        let ast = result.unwrap();
        match ast {
            ASTNode::Match {
                expression, cases, ..
            } => {
                assert_eq!(
                    *expression,
                    ASTNode::Literal {
                        value: Value::Number(Fraction::from(0)),
                        line: 1,
                        column: 9
                    }
                );
                assert_eq!(cases.len(), 1);
                assert_eq!(
                    cases[0].0,
                    ASTNode::Literal {
                        value: Value::Number(Fraction::from(1)),
                        line: 1,
                        column: 15
                    }
                );
                assert_eq!(
                    cases[0].1,
                    ASTNode::Block {
                        nodes: vec![ASTNode::Literal {
                            value: Value::Number(Fraction::from(2)),
                            line: 1,
                            column: 20
                        }],
                        line: 1,
                        column: 20
                    }
                );
            }
            _ => panic!("unexpected ast: {:?}", ast),
        }
    }
}
