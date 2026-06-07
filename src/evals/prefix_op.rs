use crate::ast::ASTNode;
use crate::environment::Env;
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::token::TokenKind;
use crate::value::Value;

pub fn prefix_op(
    op: TokenKind,
    expr: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let value = eval(*expr, env)?;
    match (op.clone(), value) {
        (TokenKind::Minus, Value::Int(v)) => Ok(Value::Int(-v)),
        (TokenKind::Minus, Value::Number(v)) => Ok(Value::from_fraction(-v)),
        _ => Err(RuntimeError::new(
            format!("Unexpected prefix op: {:?}", op).as_str(),
            line,
            column,
        )),
    }
}
