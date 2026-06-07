use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;

pub fn assign_node(
    name: String,
    value: Box<ASTNode>,
    value_type: ValueType,
    variable_type: EnvVariableType,
    is_new: bool,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let value = eval(*value, env)?;
    //let value_type = match value {
    //    Value::Number(_) => ValueType::Number,
    //    Value::String(_) => ValueType::String,
    //    Value::Bool(_) => ValueType::Bool,
    //    Value::Function => ValueType::Function,
    //    Value::Lambda { .. } => ValueType::Lambda,
    //    Value::Void => ValueType::Void,
    //    Value::Return(ref value) => {
    //        if let Value::Void = **value {
    //            ValueType::Void
    //        } else {
    //            value.value_type()
    //        }
    //    },
    //    Value::Option(_) => {
    //        ValueType::OptionType(Box::new(value_type))
    //    },
    //    Value::List(ref elements) => {
    //        if elements.len() == 0 {
    //            ValueType::List(Box::new(ValueType::Any))
    //        } else {
    //            let first_element = elements.first().unwrap();
    //            let value_type = first_element.value_type();
    //            for e in elements {
    //                if e.value_type() != value_type {
    //                    return Err(RuntimeError::new("List value type mismatch", line, column));
    //                }
    //            }
    //            ValueType::List(Box::new(value_type))
    //        }
    //    },
    //    Value::StructInstance { ref name, fields: ref instance_fields } => {
    //        match env.get_struct(&name) {
    //            Some(Value::Struct { name: _, fields, methods: _ }) => {
    //                for (field_name, value_type) in instance_fields {
    //                    if !fields.contains_key(&field_name.to_string()) {
    //                        return Err(RuntimeError::new(format!("Struct field not found: {:?}", field_name).as_str(), line, column));
    //                    }
    //                    if fields.get(&field_name.to_string()).unwrap().value_type() != value_type.value_type() {
    //                        return Err(RuntimeError::new(format!("Struct field type mismatch: {:?}", field_name).as_str(), line, column));
    //                    }
    //                }
    //            },
    //            _ => return Err(RuntimeError::new(format!("Struct not found: {:?}", name).as_str(), line, column)),
    //        };
    //        let mut field_types = HashMap::new();
    //        for (field_name, field_value) in instance_fields {
    //            field_types.insert(field_name.clone(), field_value.value_type());
    //        }
    //        ValueType::StructInstance { name: name.to_string(), fields: field_types }
    //    },
    //    _ => return Err(RuntimeError::new("Unsupported value type", line, column)),
    //};
    let result = env.set(
        name.to_string(),
        value.clone(),
        variable_type,
        value_type,
        is_new,
    );
    if result.is_err() {
        return Err(RuntimeError::new(&result.err().unwrap(), line, column));
    }
    Ok(value)
}
