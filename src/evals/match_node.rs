use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn match_node(
    expression: Box<ASTNode>,
    cases: Vec<(ASTNode, ASTNode)>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let expression_value = eval(*expression.clone(), env)?;
    let mut count = 0;
    for (pattern, body) in cases.clone() {
        env.enter_scope(format!("match-{:?}", count).to_string());
        count += 1;
        match pattern {
            ASTNode::Variable { name, .. } if name == "_" => {
                env.leave_scope();
                return Ok(eval(body, env)?);
            }
            ASTNode::Literal { value, .. } => {
                if value == expression_value {
                    let result = eval(body, env)?;
                    env.leave_scope();
                    return Ok(result);
                }
            }
            ASTNode::OptionSome { ref value, .. } => {
                if let Value::Option(Some(ref some_value)) = expression_value {
                    match value.as_ref() {
                        ASTNode::Literal { value, .. } => {
                            if value == some_value.as_ref() {
                                let result = eval(body, env)?;
                                env.leave_scope();
                                return Ok(result);
                            }
                        }
                        ASTNode::Variable { name, .. } => {
                            let _ = env.set(
                                name.clone(),
                                *some_value.clone(),
                                EnvVariableType::Immutable,
                                some_value.value_type().clone(),
                                true,
                            );
                            let result = eval(body, env)?;
                            env.leave_scope();
                            return Ok(result);
                        }
                        _ => {
                            return Err(RuntimeError::new("Unsupported pattern", line, column));
                        }
                    }
                }
            }
            ASTNode::OptionNone { .. } => {
                if let Value::Option(None) = expression_value {
                    let result = eval(body, env)?;
                    env.leave_scope();
                    return Ok(result);
                }
            }
            ASTNode::ResultSuccess { ref value, .. } => {
                let value_type = match expression.as_ref() {
                    ASTNode::Variable { value_type, .. } => match value_type {
                        Some(ValueType::ResultType { success, .. }) => success.clone(),
                        _ => {
                            panic!("Value type not found")
                        }
                    },
                    _ => {
                        panic!("Value type not found");
                    }
                };
                match value.as_ref() {
                    ASTNode::Variable { name, .. } => {
                        let expression_value = match expression_value {
                            Value::Result(Ok(ref some_value)) => some_value.clone(),
                            Value::Result(Err(ref some_value)) => some_value.clone(),
                            _ => {
                                return Err(RuntimeError::new("Unsupported pattern", line, column));
                            }
                        };
                        if expression_value.value_type() != *value_type {
                            continue;
                        }
                        let _ = env.set(
                            name.clone(),
                            *expression_value.clone(),
                            EnvVariableType::Immutable,
                            *value_type.clone(),
                            true,
                        );
                        let result = eval(body, env)?;
                        env.leave_scope();
                        return Ok(result);
                    }
                    _ => {
                        println!("not expected pattern: {:?}", value);
                    }
                }
                match expression_value {
                    Value::Result(Ok(ref some_value)) => {
                        let evaluated_value = eval(*value.clone(), env)?;
                        if evaluated_value == *some_value.clone() {
                            let result = eval(body, env)?;
                            env.leave_scope();
                            return Ok(result);
                        }
                    }
                    _ => {
                        println!("Pattern: {:?}", value);
                    }
                }
            }
            ASTNode::ResultFailure { ref value, .. } => {
                let value_type = match expression.as_ref() {
                    ASTNode::Variable { value_type, .. } => match value_type {
                        Some(ValueType::ResultType { failure, .. }) => failure.clone(),
                        _ => {
                            panic!("Failure value type not found");
                        }
                    },
                    _ => {
                        panic!("Value type not found");
                    }
                };

                match value.as_ref() {
                    ASTNode::Variable { name, .. } => {
                        let expression_value = match expression_value {
                            Value::Result(Err(ref some_value)) => some_value.clone(),
                            Value::Result(Ok(ref some_value)) => some_value.clone(),
                            _ => {
                                return Err(RuntimeError::new(
                                    "Unsupported pattern in ResultFailure",
                                    line,
                                    column,
                                ));
                            }
                        };

                        if expression_value.value_type() != *value_type {
                            continue;
                        }
                        let _ = env.set(
                            name.clone(),
                            *expression_value.clone(),
                            EnvVariableType::Immutable,
                            *value_type.clone(),
                            true,
                        );
                        let result = eval(body, env)?;
                        env.leave_scope();
                        return Ok(result);
                    }
                    _ => {
                        println!("Unexpected pattern in ResultFailure: {:?}", value);
                    }
                }

                match expression_value {
                    Value::Result(Err(ref some_value)) => {
                        let evaluated_value = eval(*value.clone(), env)?;
                        if evaluated_value == *some_value.clone() {
                            let result = eval(body, env)?;
                            env.leave_scope();
                            return Ok(result);
                        }
                    }
                    _ => {
                        println!("Pattern did not match Err: {:?}", expression_value);
                    }
                }
            }
            _ => {
                println!("Pattern");
            }
        }
        env.leave_scope();
    }
    Err(RuntimeError::new("No match found", line, column))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtins;
    use crate::evals::evals;
    use crate::parsers::Parser;
    use crate::tokenizer::tokenize;
    use fraction::Fraction;

    #[test]
    fn test_pattern_matching() {
        let input = r#"
        match 1 {
            1 => {2}
            _ => {3}
        }
        "#
        .to_string();
        let mut env = Env::new();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse().unwrap();
        let result = eval(ast, &mut env).unwrap();
        assert_eq!(result, Value::Number(Fraction::from(2)));

        let input = r#"
        match 2 {
            1 => { 2 }
            _ => { 3 }
        }
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse().unwrap();
        let result = eval(ast, &mut env).unwrap();
        assert_eq!(result, Value::Number(Fraction::from(3)));
        let input = r#"
        match Some(2) {
            Some(2) => { 2 }
            _ => { 3 }
        }
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse().unwrap();
        let result = eval(ast, &mut env).unwrap();
        assert_eq!(result, Value::Number(Fraction::from(2)));
        let input = r#"
        val x:Option<number> = Some(2)
        match x {
            Some(x) => { x + 10 }
            None => { 3 }
            _ => { 4 }
        }
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        let result = evals(ast, &mut env).unwrap();
        assert_eq!(result[1], Value::Number(Fraction::from(12)));
        let input = r#"
        match None {
            Some(2) => { 2 }
            None => { 3 }
            _ => { 4 }
        }
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse().unwrap();
        let result = eval(ast, &mut env).unwrap();
        assert_eq!(result, Value::Number(Fraction::from(3)));
    }
}
