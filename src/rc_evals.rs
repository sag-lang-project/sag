use crate::ast::ASTNode;
use crate::environment::{EnvVariableType, FunctionInfo, ValueType};
use crate::evals::runtime_error::RuntimeError;
use crate::rc_env::RcEnv;
use crate::rc_value::RcValue;
use crate::value::Value;
use fraction::Fraction;
use std::rc::Rc;

pub fn rc_evals(nodes: Vec<ASTNode>, env: &mut RcEnv) -> Result<RcValue, RuntimeError> {
    let mut result = RcValue::Void;
    for node in nodes {
        result = rc_eval(node, env)?;
        if let RcValue::Return(_) = result {
            break;
        }
    }
    Ok(result)
}

pub fn rc_eval(node: ASTNode, env: &mut RcEnv) -> Result<RcValue, RuntimeError> {
    match node {
        ASTNode::Literal { value, .. } => {
            // リテラル値をRcValueに変換
            Ok(RcValue::from_value(&value))
        }
        ASTNode::Variable {
            name,
            value_type: _,
            line,
            column,
        } => {
            // 変数の値を取得
            match env.get(&name, None) {
                Some(var_info) => Ok(var_info.value),
                None => Err(RuntimeError::new(
                    &format!("undefined variable: \"{}\"", name),
                    line,
                    column,
                )),
            }
        }
        ASTNode::BinaryOp {
            left,
            op,
            right,
            line,
            column,
        } => {
            // 二項演算
            let left_val = rc_eval(*left, env)?;
            let right_val = rc_eval(*right, env)?;
            let op_str = format!("{:?}", op);
            rc_binary_op(left_val, op_str, right_val, line, column)
        }
        ASTNode::PrefixOp {
            op,
            expr,
            line,
            column,
        } => {
            // 単項演算
            let expr_val = rc_eval(*expr, env)?;
            let op_str = format!("{:?}", op);
            rc_prefix_op(op_str, expr_val, line, column)
        }
        ASTNode::Assign {
            name,
            value,
            variable_type,
            value_type,
            is_new,
            line,
            column,
        } => {
            // 変数代入
            let value_val = rc_eval(*value, env)?;

            if is_new {
                if let Err(e) = env.set(
                    name.clone(),
                    value_val.clone(),
                    variable_type,
                    value_type,
                    true,
                ) {
                    return Err(RuntimeError::new(&e, line, column));
                }
            } else {
                if let Err(e) = env.set(
                    name.clone(),
                    value_val.clone(),
                    variable_type,
                    value_type,
                    false,
                ) {
                    return Err(RuntimeError::new(&e, line, column));
                }
            }

            Ok(value_val)
        }
        ASTNode::FunctionCall {
            name,
            arguments,
            line,
            column,
        } => rc_function_call(name, arguments, line, column, env),
        ASTNode::Function {
            name,
            arguments,
            body,
            return_type,
            ..
        } => {
            env.register_rc_function(
                name,
                crate::rc_env::RcFunctionInfo {
                    arguments,
                    return_type,
                    body: Some(*body),
                    builtin: None,
                },
            );
            Ok(RcValue::Function)
        }
        ASTNode::Block { nodes, .. } => {
            let mut last_value = RcValue::Void;
            for statement in nodes {
                let value = rc_eval(statement, env)?;
                match value {
                    RcValue::Return(_) | RcValue::Break | RcValue::Continue => return Ok(value),
                    _ => last_value = value,
                }
            }
            Ok(last_value)
        }
        ASTNode::Return { expr, .. } => Ok(RcValue::Return(Rc::new(rc_eval(*expr, env)?))),
        ASTNode::Break { .. } => Ok(RcValue::Break),
        ASTNode::Continue { .. } => Ok(RcValue::Continue),
        ASTNode::If {
            condition,
            then,
            else_,
            line,
            column,
            ..
        } => match rc_eval(*condition, env)? {
            RcValue::Bool(true) => rc_eval(*then, env),
            RcValue::Bool(false) => {
                if let Some(else_node) = else_ {
                    rc_eval(*else_node, env)
                } else {
                    Ok(RcValue::Void)
                }
            }
            other => Err(RuntimeError::new(
                format!("Condition must be a boolean: {:?}", other).as_str(),
                line,
                column,
            )),
        },
        ASTNode::For {
            variable,
            iterable,
            body,
            line,
            column,
        } => match rc_eval(*iterable, env)? {
            RcValue::List(values) => {
                let scope_name = format!("for-{}", variable);
                for value in values.iter() {
                    env.enter_scope(scope_name.clone());
                    env.set(
                        variable.clone(),
                        value.clone(),
                        EnvVariableType::Immutable,
                        value.value_type(),
                        true,
                    )
                    .map_err(|e| RuntimeError::new(&e, line, column))?;
                    let result = rc_eval((*body).clone(), env)?;
                    env.leave_scope();
                    match result {
                        RcValue::Return(_) => return Ok(result),
                        RcValue::Break => return Ok(RcValue::Void),
                        RcValue::Continue => continue,
                        _ => {}
                    }
                }
                Ok(RcValue::Void)
            }
            other => Err(RuntimeError::new(
                format!("Unexpected iterable: {:?}", other).as_str(),
                line,
                column,
            )),
        },
        ASTNode::Eq { left, right, .. } => rc_compare(*left, *right, "eq", env),
        ASTNode::Gt {
            left,
            right,
            line,
            column,
        } => rc_compare_with_numeric(*left, *right, "gt", line, column, env),
        ASTNode::Gte {
            left,
            right,
            line,
            column,
        } => rc_compare_with_numeric(*left, *right, "gte", line, column, env),
        ASTNode::Lt {
            left,
            right,
            line,
            column,
        } => rc_compare_with_numeric(*left, *right, "lt", line, column, env),
        ASTNode::Lte {
            left,
            right,
            line,
            column,
        } => rc_compare_with_numeric(*left, *right, "lte", line, column, env),
        _ => {
            // 未実装のノードタイプの場合は、一時的に通常のevalに変換して処理
            // 最終的には、すべてのノードタイプをrc_evalで直接処理するように実装する
            let mut temp_env = env.to_env();
            let result = crate::evals::eval(node, &mut temp_env)?;
            *env = RcEnv::from_env(&temp_env);
            Ok(RcValue::from_value(&result))
        }
    }
}

fn rc_function_call(
    name: String,
    arguments: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut RcEnv,
) -> Result<RcValue, RuntimeError> {
    let args_vec = match *arguments {
        ASTNode::FunctionCallArgs { args, .. } => args,
        _ => return Err(RuntimeError::new("illegal arguments", line, column)),
    };

    let mut arg_values = Vec::with_capacity(args_vec.len());
    for arg in args_vec {
        arg_values.push(rc_eval(arg, env)?);
    }

    if let Some(function_info) = env.get_rc_builtin(&name) {
        if let Some(builtin_fn) = function_info.builtin {
            return Ok(builtin_fn(arg_values));
        }
    }

    if let Some(function) = env.get_rc_function(&name) {
        let mut params_vec = Vec::new();
        for arg in &function.arguments {
            match arg {
                ASTNode::Variable {
                    name, value_type, ..
                } => params_vec.push((name.clone(), value_type.clone())),
                _ => {
                    return Err(RuntimeError::new(
                        "illegal function parameter",
                        line,
                        column,
                    ))
                }
            }
        }

        if params_vec.len() != arg_values.len() {
            return Err(RuntimeError::new(
                "does not match arguments length",
                line,
                column,
            ));
        }

        let mut local_env = env.clone();
        local_env.enter_scope(name.clone());
        for ((param_name, param_type), arg_value) in
            params_vec.into_iter().zip(arg_values.into_iter())
        {
            local_env
                .set(
                    param_name,
                    arg_value,
                    EnvVariableType::Immutable,
                    param_type.unwrap_or(ValueType::Any),
                    true,
                )
                .map_err(|e| RuntimeError::new(&e, line, column))?;
        }

        let body = function
            .body
            .ok_or_else(|| RuntimeError::new("function body missing", line, column))?;
        let result = rc_eval(body, &mut local_env)?;
        env.update_global_env(&local_env);
        return match result {
            RcValue::Return(value) => Ok((*value).clone()),
            other => Ok(other),
        };
    }

    if let Some(function) = env.get_function(&name) {
        let mut temp_env = env.to_env();
        temp_env.register_function(
            name.clone(),
            FunctionInfo {
                arguments: function.arguments,
                return_type: function.return_type,
                body: function.body,
                builtin: function.builtin,
            },
        );
        let result = crate::evals::eval(
            ASTNode::FunctionCall {
                name,
                arguments: Box::new(ASTNode::FunctionCallArgs {
                    args: arg_values
                        .into_iter()
                        .map(|v| ASTNode::Literal {
                            value: v.to_value(),
                            line,
                            column,
                        })
                        .collect(),
                    line,
                    column,
                }),
                line,
                column,
            },
            &mut temp_env,
        )?;
        *env = RcEnv::from_env(&temp_env);
        return Ok(RcValue::from_value(&result));
    }

    Err(RuntimeError::new(
        format!("Function is missing: {:?}", name).as_str(),
        line,
        column,
    ))
}

fn rc_compare(
    left: ASTNode,
    right: ASTNode,
    op: &str,
    env: &mut RcEnv,
) -> Result<RcValue, RuntimeError> {
    let left_val = rc_eval(left, env)?;
    let right_val = rc_eval(right, env)?;
    Ok(RcValue::Bool(match op {
        "eq" => left_val == right_val,
        _ => false,
    }))
}

fn rc_compare_with_numeric(
    left: ASTNode,
    right: ASTNode,
    op: &str,
    line: usize,
    column: usize,
    env: &mut RcEnv,
) -> Result<RcValue, RuntimeError> {
    let left_val = rc_eval(left, env)?;
    let right_val = rc_eval(right, env)?;
    match (&left_val, &right_val) {
        (RcValue::Number(a), RcValue::Number(b)) => Ok(RcValue::Bool(match op {
            "gt" => a > b,
            "gte" => a >= b,
            "lt" => a < b,
            "lte" => a <= b,
            _ => false,
        })),
        _ => Err(RuntimeError::new(
            "comparison expects numbers",
            line,
            column,
        )),
    }
}

// 二項演算の実装
fn rc_binary_op(
    left: RcValue,
    op: String,
    right: RcValue,
    line: usize,
    column: usize,
) -> Result<RcValue, RuntimeError> {
    match op.as_str() {
        "Plus" => match (&left, &right) {
            (RcValue::Number(a), RcValue::Number(b)) => Ok(RcValue::Number(a + b)),
            (RcValue::String(a), RcValue::String(b)) => {
                let mut new_str = a.to_string();
                new_str.push_str(&b);
                Ok(RcValue::new_string(new_str))
            }
            (RcValue::String(a), other) => {
                let mut new_str = a.to_string();
                new_str.push_str(&other.to_string());
                Ok(RcValue::new_string(new_str))
            }
            _ => Err(RuntimeError::new(
                &format!(
                    "unsupported operand types for +: {:?} and {:?}",
                    left, right
                ),
                line,
                column,
            )),
        },
        "Minus" => match (&left, &right) {
            (RcValue::Number(a), RcValue::Number(b)) => Ok(RcValue::Number(a - b)),
            _ => Err(RuntimeError::new(
                &format!(
                    "unsupported operand types for -: {:?} and {:?}",
                    left, right
                ),
                line,
                column,
            )),
        },
        "Multiply" => match (&left, &right) {
            (RcValue::Number(a), RcValue::Number(b)) => Ok(RcValue::Number(a * b)),
            _ => Err(RuntimeError::new(
                &format!(
                    "unsupported operand types for *: {:?} and {:?}",
                    left, right
                ),
                line,
                column,
            )),
        },
        "Divide" => match (&left, &right) {
            (RcValue::Number(a), RcValue::Number(b)) => {
                if *b == Fraction::from(0) {
                    Err(RuntimeError::new("division by zero", line, column))
                } else {
                    Ok(RcValue::Number(a / b))
                }
            }
            _ => Err(RuntimeError::new(
                &format!(
                    "unsupported operand types for /: {:?} and {:?}",
                    left, right
                ),
                line,
                column,
            )),
        },
        // 他の演算子も同様に実装
        _ => {
            // 未実装の演算子の場合は、一時的に通常のevalに変換して処理
            let left_val = left.to_value();
            let right_val = right.to_value();
            let result = match op.as_str() {
                "Equal" => Ok(Value::Bool(left_val == right_val)),
                "NotEqual" => Ok(Value::Bool(left_val != right_val)),
                "LessThan" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a < b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for <: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                "GreaterThan" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a > b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for >: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                "LessThanOrEqual" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a <= b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for <=: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                "GreaterThanOrEqual" => match (&left_val, &right_val) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a >= b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for >=: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                "And" => match (&left_val, &right_val) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for &&: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                "Or" => match (&left_val, &right_val) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                    _ => Err(RuntimeError::new(
                        &format!(
                            "unsupported operand types for ||: {:?} and {:?}",
                            left_val, right_val
                        ),
                        line,
                        column,
                    )),
                },
                _ => Err(RuntimeError::new(
                    &format!("unsupported operator: {}", op),
                    line,
                    column,
                )),
            }?;
            Ok(RcValue::from_value(&result))
        }
    }
}

// 単項演算の実装
fn rc_prefix_op(
    op: String,
    expr: RcValue,
    line: usize,
    column: usize,
) -> Result<RcValue, RuntimeError> {
    match op.as_str() {
        "Minus" => match expr {
            RcValue::Number(a) => Ok(RcValue::Number(-a)),
            _ => Err(RuntimeError::new(
                &format!("unsupported operand type for -: {:?}", expr),
                line,
                column,
            )),
        },
        "Not" => match expr {
            RcValue::Bool(b) => Ok(RcValue::Bool(!b)),
            _ => Err(RuntimeError::new(
                &format!("unsupported operand type for !: {:?}", expr),
                line,
                column,
            )),
        },
        // 他の演算子も同様に実装
        _ => {
            // 未実装の演算子の場合は、エラーを返す
            Err(RuntimeError::new(
                &format!("unsupported prefix operator: {}", op),
                line,
                column,
            ))
        }
    }
}
