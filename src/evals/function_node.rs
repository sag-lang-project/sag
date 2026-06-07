use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, FunctionInfo, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn function_node(
    name: String,
    arguments: Vec<ASTNode>,
    body: Box<ASTNode>,
    return_type: ValueType,
    _line: usize,
    _column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let function_info = FunctionInfo {
        arguments,
        body: Some(*body),
        return_type,
        builtin: None,
    };
    env.register_function(name, function_info);
    Ok(Value::Function)
}

pub fn block_node(
    statements: Vec<ASTNode>,
    _line: usize,
    _column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let mut last_value = Value::Void;
    for statement in statements {
        let value = eval(statement, env)?;
        if let Value::Return(v) = &value {
            return Ok(Value::Return(v.clone()));
        }
        if let Value::Break = value {
            return Ok(Value::Break);
        }
        if let Value::Continue = value {
            return Ok(Value::Continue);
        }
        last_value = value;
    }
    Ok(last_value)
}

pub fn function_call_node(
    name: String,
    arguments: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    if env.get_function(&name).is_some() || env.get_builtin(&name).is_some() {
        let function = match env.get_function(&name) {
            Some(function) => function.clone(),
            None => {
                let builtin = env.get_builtin(&name);
                if builtin.is_some() {
                    builtin.unwrap().clone()
                } else {
                    return Err(RuntimeError::new(
                        format!("Function is missing: {:?}", name).as_str(),
                        line,
                        column,
                    ));
                }
            }
        };
        let mut params_vec = vec![];
        for arg in &function.arguments {
            params_vec.push(match arg {
                ASTNode::Variable {
                    name, value_type, ..
                } => (name, value_type),
                _ => {
                    return Err(RuntimeError::new(
                        format!("illigal param: {:?}", function.arguments).as_str(),
                        line,
                        column,
                    ))
                }
            });
        }

        let args_vec = match *arguments {
            ASTNode::FunctionCallArgs {
                args: arguments, ..
            } => arguments,
            _ => {
                return Err(RuntimeError::new(
                    format!("illigal arguments: {:?}", arguments).as_str(),
                    line,
                    column,
                ))
            }
        };

        if let Some(func) = function.builtin {
            let result = func(
                args_vec
                    .iter()
                    .map(|arg| eval(arg.clone(), env))
                    .collect::<Result<Vec<Value>, RuntimeError>>()?,
            );
            return Ok(result);
        };

        if args_vec.len() != function.arguments.len() {
            return Err(RuntimeError::new(
                "does not match arguments length",
                line,
                column,
            ));
        }

        let mut local_env = env.clone();

        local_env.enter_scope(name.to_string());

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

        let result = eval(function.body.unwrap(), &mut local_env)?;
        env.update_global_env(&local_env);

        local_env.leave_scope();
        if let Value::Return(v) = result {
            Ok(*v)
        } else {
            Ok(result)
        }
    } else if env.get(&name, Some(&ValueType::Lambda)).is_some() {
        let lambda = match env.get(&name, None).unwrap().value.clone() {
            Value::Lambda {
                arguments,
                body,
                env: lambda_env,
            } => (arguments, body, lambda_env),
            _ => {
                return Err(RuntimeError::new(
                    format!("Function is missing: {:?}", name).as_str(),
                    line,
                    column,
                ))
            }
        };

        let mut params_vec = vec![];
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

        let args_vec = match *arguments {
            ASTNode::FunctionCallArgs {
                args: arguments, ..
            } => arguments,
            _ => {
                return Err(RuntimeError::new(
                    format!("illigal arguments: {:?}", arguments).as_str(),
                    line,
                    column,
                ))
            }
        };

        if args_vec.len() != lambda.0.len() {
            return Err(RuntimeError::new(
                "does not match arguments length",
                line,
                column,
            ));
        }

        let mut local_env = env.clone();

        local_env.enter_scope(name.to_string());

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

        let result = eval(*lambda.1, &mut local_env)?;

        env.update_global_env(&local_env);

        env.leave_scope();
        Ok(result)
    } else {
        Err(RuntimeError::new(
            format!("Function is missing: {:?}", name).as_str(),
            line,
            column,
        ))
    }
}
