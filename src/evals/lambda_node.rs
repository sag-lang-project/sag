use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn lambda_call_node(
    lambda: Box<ASTNode>,
    arguments: Vec<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let mut params_vec = vec![];
    let lambda = match *lambda {
        ASTNode::Lambda {
            arguments, body, ..
        } => (arguments, body),
        _ => {
            return Err(RuntimeError::new(
                format!("Unexpected value type: {:?}", lambda).as_str(),
                line,
                column,
            ))
        }
    };
    for arg in &lambda.0 {
        params_vec.push(match arg {
            ASTNode::Variable {
                name, value_type, ..
            } => (name, value_type),
            _ => {
                return Err(RuntimeError::new(
                    format!("illigal param: {:?}", lambda.0).as_str(),
                    line,
                    column,
                ))
            }
        });
    }

    let mut args_vec = vec![];

    for arg in arguments {
        match arg {
            ASTNode::FunctionCallArgs {
                args: arguments, ..
            } => {
                args_vec = arguments;
            }
            _ => {
                args_vec.push(arg);
            }
        }
    }
    if args_vec.len() != lambda.0.len() {
        return Err(RuntimeError::new(
            format!(
                "does not match arguments length: expected {}, got {}",
                lambda.0.len(),
                args_vec.len()
            )
            .as_str(),
            line,
            column,
        ));
    }

    let mut local_env = env.clone();

    local_env.enter_scope("lambda".to_string());

    for (param, arg) in params_vec.iter().zip(&args_vec) {
        let arg_value = eval(arg.clone(), env)?;
        let name = param.0.to_string();
        let value_type = param.1.clone();
        let _ = local_env.set(
            name,
            arg_value,
            EnvVariableType::Immutable,
            value_type.unwrap_or(ValueType::Any),
            true,
        );
    }

    let result = eval(*lambda.1, &mut local_env);

    env.update_global_env(&local_env);

    env.leave_scope();
    result
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
    fn test_lambda_expression() {
        let mut env = Env::new();
        let input = r#"
        |3, 4| -> \|x: number, y: number| => x + y
        "#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines().unwrap();
        let result = evals(asts, &mut env).unwrap();
        assert_eq!(*result.last().unwrap(), Value::Number(Fraction::from(7)));
    }
}
