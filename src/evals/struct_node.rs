use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, MethodInfo, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;
use std::collections::HashMap;

pub fn struct_node(
    name: String,
    fields: HashMap<String, ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let mut struct_fields = HashMap::new();
    // fields field_name: StructField
    for (field_name, struct_field) in fields {
        match struct_field {
            ASTNode::StructField {
                value_type,
                is_public,
                ..
            } => {
                struct_fields.insert(
                    field_name,
                    Value::StructField {
                        value_type,
                        is_public,
                    },
                );
            }
            _ => {
                return Err(RuntimeError::new(
                    format!("Unexpected struct field: {:?}", struct_field).as_str(),
                    line,
                    column,
                ))
            }
        }
    }
    let result = Value::Struct {
        name,
        fields: struct_fields,
        methods: HashMap::new(),
    };
    env.register_struct(result.clone())?;
    Ok(result)
}

pub fn impl_node(
    base_struct: Box<ValueType>,
    methods: Vec<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let mut impl_methods = HashMap::new();
    for method in methods {
        match method {
            ASTNode::Method {
                name,
                arguments,
                body,
                return_type,
                is_mut,
                ..
            } => {
                let method_info = MethodInfo {
                    arguments,
                    body: Some(*body),
                    return_type,
                    is_mut,
                };
                impl_methods.insert(name, method_info);
            }
            _ => {
                return Err(RuntimeError::new(
                    format!("Unexpected method: {:?}", method).as_str(),
                    line,
                    column,
                ))
            }
        }
    }
    let result = Value::Impl {
        base_struct: *base_struct,
        methods: impl_methods,
    };
    env.register_impl(result.clone())?;
    Ok(result)
}

pub fn struct_instance_node(
    name: String,
    fields: HashMap<String, ASTNode>,
    _line: usize,
    _column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let mut struct_fields = HashMap::new();
    for (field_name, field_value) in fields {
        struct_fields.insert(field_name, eval(field_value, env)?);
    }
    Ok(Value::StructInstance {
        name,
        fields: struct_fields,
    })
}

pub fn struct_field_assign_node(
    instance: Box<ASTNode>,
    updated_field_name: String,
    updated_value_ast: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    match *instance {
        ASTNode::StructFieldAccess {
            instance,
            field_name: _,
            line,
            column,
        } => match *instance {
            ASTNode::Variable {
                name: variable_name,
                value_type,
                line,
                column,
            } => match value_type {
                Some(ValueType::Struct { name, fields, .. }) if variable_name == "self" => {
                    match env.get_struct(&name) {
                        Some(Value::Struct {
                            fields: _, methods, ..
                        }) => {
                            let scope = env.get_current_scope();
                            match methods.get(&scope) {
                                Some(MethodInfo { arguments, .. }) => {
                                    let first_argument = arguments.first();
                                    if first_argument.is_none() {
                                        return Err(RuntimeError::new(
                                            format!("{} is missing self argument", scope).as_str(),
                                            line,
                                            column,
                                        ));
                                    }
                                    match first_argument.unwrap() {
                                        ASTNode::Variable {
                                            name: self_argument,
                                            value_type: self_type,
                                            line,
                                            column,
                                        } => {
                                            if self_argument != "self"
                                                || *self_type != Some(ValueType::MutSelfType)
                                            {
                                                return Err(RuntimeError::new(
                                                    format!("{} is not mut self argument", scope)
                                                        .as_str(),
                                                    *line,
                                                    *column,
                                                ));
                                            }
                                        }
                                        _ => {
                                            return Err(RuntimeError::new(
                                                format!("{} is missing self argument", scope)
                                                    .as_str(),
                                                line,
                                                column,
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!("{} is missing self argument", scope).as_str(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(RuntimeError::new(
                                format!("Struct not found: {:?}", name).as_str(),
                                line,
                                column,
                            ));
                        }
                    };
                    let obj = env.get(&variable_name, None);
                    if obj.is_none() {
                        return Err(RuntimeError::new(
                            format!("Variable not found: {:?}", variable_name).as_str(),
                            line,
                            column,
                        ));
                    }
                    let mut struct_fields = HashMap::new();
                    match obj.unwrap().value.clone() {
                        Value::StructInstance { .. } => {
                            let instance_value = obj.unwrap().value.clone();
                            let updated_value = match instance_value {
                                Value::StructInstance { name, fields } => {
                                    let mut updated_fields = fields.clone();
                                    let updated_value = eval(*updated_value_ast.clone(), env)?;
                                    *updated_fields
                                        .entry(updated_field_name.to_string())
                                        .or_insert(updated_value.clone()) = updated_value.clone();
                                    Value::StructInstance {
                                        name,
                                        fields: updated_fields,
                                    }
                                }
                                _ => {
                                    return Err(RuntimeError::new(
                                        format!("Unexpected value type: {:?}", instance_value)
                                            .as_str(),
                                        line,
                                        column,
                                    ))
                                }
                            };
                            env.set(
                                variable_name.to_string(),
                                updated_value.clone(),
                                EnvVariableType::Mutable,
                                ValueType::StructInstance {
                                    name: name.to_string(),
                                    fields: fields.clone(),
                                },
                                false,
                            )
                            .expect("update variable");
                            Ok(updated_value)
                        }
                        Value::Struct {
                            name: _,
                            fields: obj_fields,
                            ..
                        } => {
                            for (field_name, field_value) in obj_fields {
                                if field_name == updated_field_name {
                                    let updated_value = eval(*updated_value_ast.clone(), env)?;
                                    if field_value.value_type() != updated_value.value_type() {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Struct field type mismatch: {}.{}:{:?} = {:?}",
                                                variable_name,
                                                field_name,
                                                field_value.value_type(),
                                                updated_value.value_type()
                                            )
                                            .as_str(),
                                            line,
                                            column,
                                        ));
                                    }
                                    struct_fields.insert(field_name, updated_value);
                                } else {
                                    struct_fields.insert(field_name, field_value);
                                }
                            }
                            let env_updated_result = env.set(
                                variable_name.to_string(),
                                Value::StructInstance {
                                    name: variable_name.to_string(),
                                    fields: struct_fields.clone(),
                                },
                                EnvVariableType::Mutable,
                                ValueType::StructInstance {
                                    name: name.to_string(),
                                    fields: fields.clone(),
                                },
                                false,
                            );
                            if env_updated_result.is_err() {
                                return Err(RuntimeError::new(
                                    format!("{}", env_updated_result.unwrap_err()).as_str(),
                                    line,
                                    column,
                                ));
                            }
                            Ok(Value::StructInstance {
                                name: variable_name.to_string(),
                                fields: struct_fields,
                            })
                        }
                        _ => Err(RuntimeError::new(
                            format!("Unexpected value type: {:?}", obj).as_str(),
                            line,
                            column,
                        )),
                    }
                }
                Some(ValueType::StructInstance { name, fields }) => {
                    let obj = env.get(
                        &variable_name,
                        Some(&ValueType::StructInstance {
                            name: name.to_string(),
                            fields: fields.clone(),
                        }),
                    );
                    if obj.is_none() {
                        return Err(RuntimeError::new(
                            format!("Variable not found: {:?}", variable_name).as_str(),
                            line,
                            column,
                        ));
                    }
                    let mut struct_fields = HashMap::new();
                    match obj.unwrap().value.clone() {
                        Value::StructInstance {
                            name: _,
                            fields: obj_fields,
                        } => {
                            for (field_name, field_value) in obj_fields {
                                if field_name == updated_field_name {
                                    let updated_value = eval(*updated_value_ast.clone(), env)?;
                                    if field_value.value_type() != updated_value.value_type() {
                                        return Err(RuntimeError::new(
                                            format!(
                                                "Struct field type mismatch: {}.{}:{:?} = {:?}",
                                                variable_name,
                                                field_name,
                                                field_value.value_type(),
                                                updated_value.value_type()
                                            )
                                            .as_str(),
                                            line,
                                            column,
                                        ));
                                    }
                                    struct_fields.insert(field_name, updated_value);
                                } else {
                                    struct_fields.insert(field_name, field_value);
                                }
                            }
                            let env_updated_result = env.set(
                                variable_name.to_string(),
                                Value::StructInstance {
                                    name: variable_name.to_string(),
                                    fields: struct_fields.clone(),
                                },
                                EnvVariableType::Mutable,
                                ValueType::StructInstance {
                                    name: name.to_string(),
                                    fields: fields.clone(),
                                },
                                false,
                            );
                            if env_updated_result.is_err() {
                                return Err(RuntimeError::new(
                                    format!("{}", env_updated_result.unwrap_err()).as_str(),
                                    line,
                                    column,
                                ));
                            }
                            Ok(Value::StructInstance {
                                name: variable_name.to_string(),
                                fields: struct_fields,
                            })
                        }
                        _ => Err(RuntimeError::new(
                            format!("Unexpected value type: {:?}", obj).as_str(),
                            line,
                            column,
                        )),
                    }
                }
                _ => Err(RuntimeError::new(
                    format!("Unexpected value type: {:?}", value_type).as_str(),
                    line,
                    column,
                )),
            },
            _ => Err(RuntimeError::new(
                format!("Unexpected value type: {:?}", instance).as_str(),
                line,
                column,
            )),
        },
        _ => Err(RuntimeError::new(
            format!("Unexpected value type: {:?}", instance).as_str(),
            line,
            column,
        )),
    }
}

pub fn struct_field_access_node(
    instance: Box<ASTNode>,
    field_name: String,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let struct_obj = match *instance {
        ASTNode::Variable {
            name: variable_name,
            value_type,
            line,
            column,
        } => match value_type {
            Some(ValueType::Struct { .. }) if variable_name == "self" => {
                let obj = env.get(&variable_name, None);
                if obj.is_none() {
                    return Err(RuntimeError::new(
                        format!("Variable not found: {:?}", variable_name).as_str(),
                        line,
                        column,
                    ));
                }
                obj.unwrap().value.clone()
            }
            Some(ValueType::StructInstance { name, fields }) => {
                let obj = env.get(
                    &variable_name,
                    Some(&ValueType::StructInstance {
                        name: name.to_string(),
                        fields,
                    }),
                );
                if obj.is_none() {
                    return Err(RuntimeError::new(
                        format!("Variable not found: {:?}", variable_name).as_str(),
                        line,
                        column,
                    ));
                }
                obj.unwrap().value.clone()
            }
            _ => {
                return Err(RuntimeError::new(
                    format!("Unexpected value type: {:?}", value_type).as_str(),
                    line,
                    column,
                ))
            }
        },
        _ => {
            return Err(RuntimeError::new(
                format!("Unexpected value type: {:?}", instance).as_str(),
                line,
                column,
            ));
        }
    };
    match struct_obj {
        Value::Struct { fields, .. } => {
            // selfのケース
            if !fields.contains_key(&field_name) {
                return Err(RuntimeError::new(
                    format!("Field not found: {:?}", field_name).as_str(),
                    line,
                    column,
                ));
            }
            Ok(fields.get(&field_name).unwrap().clone())
        }
        Value::StructInstance { name: _, fields } => {
            if !fields.contains_key(&field_name) {
                return Err(RuntimeError::new(
                    format!("Field not found: {:?}", field_name).as_str(),
                    line,
                    column,
                ));
            }
            Ok(fields.get(&field_name).unwrap().clone())
        }
        _ => Err(RuntimeError::new(
            format!("Unexpected value: {:?}", struct_obj).as_str(),
            line,
            column,
        )),
    }
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
    fn test_mutset_impl() {
        let input = r#"
            struct Foo {
              value: number,
            }
            
            impl Foo {
              fun set(mut self, num: number) {
                self.value = num
              }
            }
            
            val mut foo = Foo{value: 1}
            foo.set(3)
            foo.value
        "#;

        let tokens = tokenize(&input.to_string());
        let mut env = Env::new();
        let builtins = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtins).parse_lines();
        let result = evals(asts.unwrap(), &mut env).unwrap();
        assert_eq!(result.last(), Some(&Value::Number(Fraction::from(3))));
    }

    #[test]
    fn test_not_mut_set_impl() {
        let input = r#"
            struct Foo {
              value: number,
            }
            
            impl Foo {
              fun set(self, num: number) {
                self.value = num
              }
            }
            
            val mut foo = Foo{value: 1}
            foo.set(3)
            foo.value
        "#;

        let tokens = tokenize(&input.to_string());
        let mut env = Env::new();
        let builtins = register_builtins(&mut env);
        let asts = Parser::new(tokens.to_vec(), builtins).parse_lines();
        let result = evals(asts.unwrap(), &mut env);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_not_mut_instance_impl() {
        let input = r#"
            struct Foo {
              value: number,
            }
            
            impl Foo {
              fun set(self, num: number) {
                self.value = num
              }
            }
            
            val foo = Foo{value: 1}
            foo.set(3)
            foo.value
        "#;

        let tokens = tokenize(&input.to_string());
        let mut env = Env::new();
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens.to_vec(), builtin).parse_lines();
        let result = evals(asts.unwrap(), &mut env);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_struct_impl() {
        let input = r#"
struct Point {
  x: number,
  y: number
}

impl Point {
  fun move(mut self, dx: number, dy: number) {
      self.x = self.x + dx
      self.y = self.y + dy
      Void
  }
}

impl Point {
  fun clear(mut self) {
      self.x = 0
      self.y = 0
      Void
  }
}

val x = 8
val y = 3
val mut point = Point{x: x, y: y}
point.move(5, 2)
point.clear()
"#;

        let tokens = tokenize(&input.to_string());
        let mut env = Env::new();
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines();
        let result = evals(asts.unwrap(), &mut env).unwrap();
        let base_struct = Value::Struct {
            name: "Point".into(),
            fields: HashMap::from_iter(vec![
                (
                    "y".into(),
                    Value::StructField {
                        value_type: ValueType::Number,
                        is_public: false,
                    },
                ),
                (
                    "x".into(),
                    Value::StructField {
                        value_type: ValueType::Number,
                        is_public: false,
                    },
                ),
            ]),
            methods: HashMap::new(),
        };
        assert_eq!(result.first(), Some(base_struct.clone()).as_ref());
        assert_eq!(result.get(6), Some(Value::Void).as_ref());
    }

    #[test]
    fn test_struct_other_type_assign() {
        let input = r#"
            struct Point {
                x: number,
                y: number
            }
            val mut point = Point{x: 1, y: 2}
            point.x = "hello"
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines().unwrap();
        assert_eq!(evals(asts, &mut env).is_err(), true);
    }

    #[test]
    fn test_struct_access() {
        let input = r#"
            struct Point {
                x: number,
                y: number
            }
            val mut point = Point{x: 1, y: 2}
            point.x
            point.x = 3
            point.x
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines().unwrap();
        let result = evals(asts, &mut env).unwrap();
        assert_eq!(result[4], Value::Number(Fraction::from(3)));
    }

    #[test]
    fn test_assign_struct() {
        let mut env = Env::new();
        let input = r#"
            struct Point {
                x: number,
                y: number
            }
            val mut point = Point{x: 1, y: 2}
            point
        "#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines().unwrap();
        let result = evals(asts, &mut env).unwrap();
        assert_eq!(
            result.last(),
            Some(&Value::StructInstance {
                name: "Point".to_string(),
                fields: HashMap::from_iter(vec![
                    ("x".to_string(), Value::Number(Fraction::from(1))),
                    ("y".to_string(), Value::Number(Fraction::from(2))),
                ])
            })
        );
    }

    #[test]
    fn test_struct() {
        let mut env = Env::new();
        let input = r#"
            struct Point {
                x: number,
                y: number
            }
        "#;
        let tokens = tokenize(&input.to_string());
        let builtin = register_builtins(&mut env);
        let asts = Parser::new(tokens, builtin).parse_lines().unwrap();
        let _ = evals(asts, &mut env);
        assert_eq!(env.get_struct(&"Point".to_string()).is_some(), true);
        assert_eq!(env.get_struct(&"DummuStruct".to_string()).is_some(), false);
    }
}
