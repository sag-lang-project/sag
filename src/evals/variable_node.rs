use crate::environment::{Env, ValueType};
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn variable_node(
    name: String,
    _value_type: Option<ValueType>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let value = env.get(&name, None);
    if value.is_none() {
        Err(RuntimeError::new(
            format!("Variable not found: {:?}", name).as_str(),
            line,
            column,
        ))
    } else {
        Ok(value.unwrap().value.clone())
    }
}
