use crate::ast::ASTNode;
use crate::environment::Env;
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::token::TokenKind;
use crate::value::Value;

pub fn comparison_op_node(
    op: TokenKind,
    left: Box<ASTNode>,
    right: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let left_value = eval(*left, env)?;
    let right_value = eval(*right, env)?;
    match (left_value, right_value, op) {
        (Value::Number(l), Value::Number(r), TokenKind::Eq) => Ok(Value::Bool(l == r)),
        (Value::Number(l), Value::Number(r), TokenKind::Neq) => Ok(Value::Bool(l != r)),
        (Value::Number(l), Value::Number(r), TokenKind::Gte) => Ok(Value::Bool(l >= r)),
        (Value::Number(l), Value::Number(r), TokenKind::Gt) => Ok(Value::Bool(l > r)),
        (Value::Number(l), Value::Number(r), TokenKind::Lte) => Ok(Value::Bool(l <= r)),
        (Value::Number(l), Value::Number(r), TokenKind::Lt) => Ok(Value::Bool(l < r)),
        (Value::String(l), Value::String(r), TokenKind::Eq) => Ok(Value::Bool(l == r)),
        (Value::String(l), Value::String(r), TokenKind::Neq) => Ok(Value::Bool(l != r)),
        (Value::Bool(l), Value::Bool(r), TokenKind::Eq) => Ok(Value::Bool(l == r)),
        (Value::Bool(l), Value::Bool(r), TokenKind::Neq) => Ok(Value::Bool(l != r)),
        _ => Err(RuntimeError::new("Unsupported operation", line, column)),
    }
}
