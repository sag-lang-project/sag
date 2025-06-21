use std::rc::Rc;
use std::collections::HashMap;
use crate::value::Value;
use crate::environment::{Env, ValueType, EnvVariableType, EnvVariableValueInfo};
use crate::ast::ASTNode;
use crate::shared_env::SharedEnv;

/// Utility functions to help with the transition to Rc-based implementation

/// Convert a traditional Value to an Rc-based Value
pub fn convert_to_rc_value(value: Value) -> Value {
    match value {
        Value::String(s) => Value::new_string(s),
        Value::List(list) => Value::new_list(list),
        Value::Dict(dict) => Value::new_dict(dict),
        Value::Struct { name, fields, methods } => Value::new_struct(name, fields, methods),
        Value::StructInstance { name, fields } => Value::new_struct_instance(name, fields),
        Value::Lambda { arguments, body, env } => Value::new_lambda(arguments, *body, env),
        Value::Option(opt) => Value::new_option(opt.map(|v| *v)),
        Value::Result(res) => match res {
            Ok(v) => Value::new_result_ok(*v),
            Err(v) => Value::new_result_err(*v),
        },
        Value::Return(v) => Value::new_return(*v),
        // Simple types don't need conversion
        _ => value,
    }
}

/// Convert an Rc-based Value to a traditional Value (for backward compatibility)
pub fn convert_from_rc_value(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String((*s).clone()),
        Value::List(list) => Value::List((*list).clone()),
        Value::Dict(dict) => Value::Dict((*dict).clone()),
        Value::Struct { name, fields, methods } => Value::Struct {
            name: (*name).clone(),
            fields: (*fields).clone(),
            methods: (*methods).clone(),
        },
        Value::StructInstance { name, fields } => Value::StructInstance {
            name: (*name).clone(),
            fields: (*fields).clone(),
        },
        Value::Lambda { arguments, body, env } => Value::Lambda {
            arguments: (*arguments).clone(),
            body: Box::new((*body).clone()),
            env: (*env).clone(),
        },
        Value::Option(opt) => Value::Option(opt.map(|v| Box::new((*v).clone()))),
        Value::Result(res) => match res {
            Ok(v) => Value::Result(Ok(Box::new((*v).clone()))),
            Err(v) => Value::Result(Err(Box::new((*v).clone()))),
        },
        Value::Return(v) => Value::Return(Box::new((*v).clone())),
        // Simple types don't need conversion
        _ => value,
    }
}

/// Helper function to modify a list value without cloning the entire list
pub fn modify_list<F>(list_value: &Value, modifier: F) -> Value
where
    F: FnOnce(&mut Vec<Value>),
{
    match list_value {
        Value::List(list_rc) => {
            let mut list = (*list_rc).clone();
            modifier(&mut list);
            Value::new_list(list)
        }
        _ => panic!("Expected a list value"),
    }
}

/// Helper function to modify a dictionary value without cloning the entire dictionary
pub fn modify_dict<F>(dict_value: &Value, modifier: F) -> Value
where
    F: FnOnce(&mut HashMap<String, Value>),
{
    match dict_value {
        Value::Dict(dict_rc) => {
            let mut dict = (*dict_rc).clone();
            modifier(&mut dict);
            Value::new_dict(dict)
        }
        _ => panic!("Expected a dictionary value"),
    }
}

/// Helper function to modify a struct instance without cloning the entire struct
pub fn modify_struct_instance<F>(struct_value: &Value, modifier: F) -> Value
where
    F: FnOnce(&mut HashMap<String, Value>),
{
    match struct_value {
        Value::StructInstance { name, fields } => {
            let mut fields_map = (*fields).clone();
            modifier(&mut fields_map);
            Value::new_struct_instance(name.as_ref().clone(), fields_map)
        }
        _ => panic!("Expected a struct instance"),
    }
}

/// Helper function to safely access a field in a struct instance
pub fn get_struct_field(struct_value: &Value, field_name: &str) -> Option<Value> {
    match struct_value {
        Value::StructInstance { fields, .. } => {
            fields.get(field_name).cloned()
        }
        _ => None,
    }
}

/// Helper function to safely access an element in a list
pub fn get_list_element(list_value: &Value, index: usize) -> Option<Value> {
    match list_value {
        Value::List(list) => {
            if index < list.len() {
                Some(list[index].clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper function to safely access a value in a dictionary
pub fn get_dict_value(dict_value: &Value, key: &str) -> Option<Value> {
    match dict_value {
        Value::Dict(dict) => {
            dict.get(key).cloned()
        }
        _ => None,
    }
}

/// Helper function to convert an environment to a shared environment
pub fn env_to_shared(env: &Env) -> SharedEnv {
    SharedEnv::from_env(env)
}

/// Helper function to convert a shared environment to a traditional environment
pub fn shared_to_env(shared_env: &SharedEnv) -> Env {
    shared_env.to_env()
}
