use crate::ast::ASTNode;
use crate::environment::{Env, EnvVariableType, ValueType};
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::value::Value;
use fraction::Fraction;
use std::collections::HashMap;

fn extract_arguments(arguments: Box<ASTNode>) -> Vec<ASTNode> {
    match *arguments {
        ASTNode::FunctionCallArgs { args, .. } => args,
        _ => vec![],
    }
}

// number builtin method
fn call_builtin_method_on_number(
    num: Fraction,
    method_name: &str,
    _args: &[ASTNode],
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    match method_name {
        "to_string" => Ok(Value::String(num.to_string())),
        "round" => Ok(Value::Number(num.round().into())),
        "sqrt" => {
            let num_f64 = *num.numer().unwrap() as f64;
            let denom_f64 = *num.denom().unwrap() as f64;
            let fraction_value = num_f64 / denom_f64;
            let sqrt_value = fraction_value.sqrt();
            Ok(Value::Number(sqrt_value.into()))
        }
        _ => Err(RuntimeError::new(
            format!("{} is not a method of number", method_name).as_str(),
            line,
            column,
        )),
    }
}

// list builtin method
fn call_builtin_method_on_list(
    mut list: Vec<Value>,
    method_name: &str,
    args: &[ASTNode],
    caller_ast: &ASTNode,
    env: &mut Env,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    match method_name {
        "to_string" => Ok(Value::String(format!("{:?}", list))),
        "push" => {
            if args.len() < 1 {
                return Err(RuntimeError::new("push requires an argument", line, column));
            }
            let new_val = eval(args[0].clone(), env)?;
            list.push(new_val);

            if let ASTNode::Variable {
                name, value_type, ..
            } = caller_ast
            {
                let result = env.set(
                    name.to_string(),
                    Value::List(list.clone()),
                    EnvVariableType::Mutable,
                    value_type.clone().unwrap_or(ValueType::Any),
                    false,
                );
                if let Err(e) = result {
                    return Err(RuntimeError::new(e.as_str(), line, column));
                }
            }
            Ok(Value::List(list))
        }
        "pop" => {
            let popped = list.pop();
            if let ASTNode::Variable {
                name, value_type, ..
            } = caller_ast
            {
                let result = env.set(
                    name.to_string(),
                    Value::List(list.clone()),
                    EnvVariableType::Mutable,
                    value_type.clone().unwrap_or(ValueType::Any),
                    false,
                );
                if let Err(e) = result {
                    return Err(RuntimeError::new(e.as_str(), line, column));
                }
            }
            Ok(Value::Option(popped.map(Box::new)))
        }
        "len" => Ok(Value::Number(Fraction::from(list.len()))),
        "is_empty" => Ok(Value::Bool(list.is_empty())),
        "first" => Ok(Value::Option(list.first().cloned().map(Box::new))),
        "last" => Ok(Value::Option(list.last().cloned().map(Box::new))),
        "clear" => {
            list.clear();
            if let ASTNode::Variable {
                name, value_type, ..
            } = caller_ast
            {
                let result = env.set(
                    name.to_string(),
                    Value::List(list.clone()),
                    EnvVariableType::Mutable,
                    value_type.clone().unwrap_or(ValueType::Any),
                    false,
                );
                if let Err(e) = result {
                    return Err(RuntimeError::new(e.as_str(), line, column));
                }
            }
            Ok(Value::Void)
        }
        "contains" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "contains requires an argument",
                    line,
                    column,
                ));
            }
            let search_val = eval(args[0].clone(), env)?;
            Ok(Value::Bool(list.contains(&search_val)))
        }
        "reverse" => {
            list.reverse();
            if let ASTNode::Variable {
                name, value_type, ..
            } = caller_ast
            {
                let result = env.set(
                    name.to_string(),
                    Value::List(list.clone()),
                    EnvVariableType::Mutable,
                    value_type.clone().unwrap_or(ValueType::Any),
                    false,
                );
                if let Err(e) = result {
                    return Err(RuntimeError::new(e.as_str(), line, column));
                }
            }
            Ok(Value::Void)
        }
        _ => Err(RuntimeError::new(
            format!("{} is not a method of list", method_name).as_str(),
            line,
            column,
        )),
    }
}

// dict builtin method
fn call_builtin_method_on_dict(
    mut dict: HashMap<String, Value>,
    method_name: &str,
    args: &[ASTNode],
    caller_ast: &ASTNode,
    env: &mut Env,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    match method_name {
        "get" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "get requires a key argument",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            if let Value::String(key) = key_val {
                Ok(Value::Option(dict.get(&key).cloned().map(Box::new)))
            } else {
                Err(RuntimeError::new("dict key must be a string", line, column))
            }
        }
        "insert" => {
            if args.len() < 2 {
                return Err(RuntimeError::new(
                    "insert requires key and value arguments",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            let value_val = eval(args[1].clone(), env)?;
            if let Value::String(key) = key_val {
                let old_value = dict.insert(key, value_val);
                if let ASTNode::Variable {
                    name, value_type, ..
                } = caller_ast
                {
                    let result = env.set(
                        name.to_string(),
                        Value::Dict(dict.clone()),
                        EnvVariableType::Mutable,
                        value_type.clone().unwrap_or(ValueType::Any),
                        false,
                    );
                    if let Err(e) = result {
                        return Err(RuntimeError::new(e.as_str(), line, column));
                    }
                }
                Ok(Value::Option(old_value.map(Box::new)))
            } else {
                Err(RuntimeError::new("dict key must be a string", line, column))
            }
        }
        "remove" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "remove requires a key argument",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            if let Value::String(key) = key_val {
                let removed_value = dict.remove(&key);
                if let ASTNode::Variable {
                    name, value_type, ..
                } = caller_ast
                {
                    let result = env.set(
                        name.to_string(),
                        Value::Dict(dict.clone()),
                        EnvVariableType::Mutable,
                        value_type.clone().unwrap_or(ValueType::Any),
                        false,
                    );
                    if let Err(e) = result {
                        return Err(RuntimeError::new(e.as_str(), line, column));
                    }
                }
                Ok(Value::Option(removed_value.map(Box::new)))
            } else {
                Err(RuntimeError::new("dict key must be a string", line, column))
            }
        }
        "contains_key" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "contains_key requires a key argument",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            if let Value::String(key) = key_val {
                Ok(Value::Bool(dict.contains_key(&key)))
            } else {
                Err(RuntimeError::new("dict key must be a string", line, column))
            }
        }
        "keys" => {
            let keys: Vec<Value> = dict.keys().map(|k| Value::String(k.clone())).collect();
            Ok(Value::List(keys))
        }
        "values" => {
            let values: Vec<Value> = dict.values().cloned().collect();
            Ok(Value::List(values))
        }
        "len" => Ok(Value::Number(Fraction::from(dict.len()))),
        "is_empty" => Ok(Value::Bool(dict.is_empty())),
        "clear" => {
            dict.clear();
            if let ASTNode::Variable {
                name, value_type, ..
            } = caller_ast
            {
                let result = env.set(
                    name.to_string(),
                    Value::Dict(dict.clone()),
                    EnvVariableType::Mutable,
                    value_type.clone().unwrap_or(ValueType::Any),
                    false,
                );
                if let Err(e) = result {
                    return Err(RuntimeError::new(e.as_str(), line, column));
                }
            }
            Ok(Value::Void)
        }
        "update" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "update requires a dictionary argument",
                    line,
                    column,
                ));
            }
            let other_dict_val = eval(args[0].clone(), env)?;
            if let Value::Dict(other_dict) = other_dict_val {
                for (key, value) in other_dict {
                    dict.insert(key, value);
                }
                if let ASTNode::Variable {
                    name, value_type, ..
                } = caller_ast
                {
                    let result = env.set(
                        name.to_string(),
                        Value::Dict(dict.clone()),
                        EnvVariableType::Mutable,
                        value_type.clone().unwrap_or(ValueType::Any),
                        false,
                    );
                    if let Err(e) = result {
                        return Err(RuntimeError::new(e.as_str(), line, column));
                    }
                }
                Ok(Value::Void)
            } else {
                Err(RuntimeError::new(
                    "update argument must be a dictionary",
                    line,
                    column,
                ))
            }
        }
        "entry" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "entry requires a key argument",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            if let Value::String(key) = key_val {
                if dict.contains_key(&key) {
                    Ok(Value::Option(dict.get(&key).cloned().map(Box::new)))
                } else {
                    Ok(Value::Option(None))
                }
            } else {
                Err(RuntimeError::new(
                    "entry key must be a string",
                    line,
                    column,
                ))
            }
        }
        "get_or_insert" => {
            if args.len() < 2 {
                return Err(RuntimeError::new(
                    "get_or_insert requires key and default value arguments",
                    line,
                    column,
                ));
            }
            let key_val = eval(args[0].clone(), env)?;
            let default_val = eval(args[1].clone(), env)?;
            if let Value::String(key) = key_val {
                let result_value = if dict.contains_key(&key) {
                    dict.get(&key).unwrap().clone()
                } else {
                    dict.insert(key.clone(), default_val.clone());
                    if let ASTNode::Variable {
                        name, value_type, ..
                    } = caller_ast
                    {
                        let result = env.set(
                            name.to_string(),
                            Value::Dict(dict.clone()),
                            EnvVariableType::Mutable,
                            value_type.clone().unwrap_or(ValueType::Any),
                            false,
                        );
                        if let Err(e) = result {
                            return Err(RuntimeError::new(e.as_str(), line, column));
                        }
                    }
                    default_val
                };
                Ok(result_value)
            } else {
                Err(RuntimeError::new(
                    "get_or_insert key must be a string",
                    line,
                    column,
                ))
            }
        }
        _ => Err(RuntimeError::new(
            format!("{} is not a method of dict", method_name).as_str(),
            line,
            column,
        )),
    }
}

// string builtin method
fn call_builtin_method_on_string(
    string: String,
    method_name: &str,
    args: &[ASTNode],
    env: &mut Env,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    match method_name {
        "len" => Ok(Value::Number(Fraction::from(string.len()))),
        "is_empty" => Ok(Value::Bool(string.is_empty())),
        "to_uppercase" => Ok(Value::String(string.to_uppercase())),
        "to_lowercase" => Ok(Value::String(string.to_lowercase())),
        "trim" => Ok(Value::String(string.trim().to_string())),
        "contains" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "contains requires a substring argument",
                    line,
                    column,
                ));
            }
            let search_val = eval(args[0].clone(), env)?;
            if let Value::String(search_str) = search_val {
                Ok(Value::Bool(string.contains(&search_str)))
            } else {
                Err(RuntimeError::new(
                    "contains argument must be a string",
                    line,
                    column,
                ))
            }
        }
        "starts_with" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "starts_with requires a prefix argument",
                    line,
                    column,
                ));
            }
            let prefix_val = eval(args[0].clone(), env)?;
            if let Value::String(prefix) = prefix_val {
                Ok(Value::Bool(string.starts_with(&prefix)))
            } else {
                Err(RuntimeError::new(
                    "starts_with argument must be a string",
                    line,
                    column,
                ))
            }
        }
        "ends_with" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "ends_with requires a suffix argument",
                    line,
                    column,
                ));
            }
            let suffix_val = eval(args[0].clone(), env)?;
            if let Value::String(suffix) = suffix_val {
                Ok(Value::Bool(string.ends_with(&suffix)))
            } else {
                Err(RuntimeError::new(
                    "ends_with argument must be a string",
                    line,
                    column,
                ))
            }
        }
        "split" => {
            if args.len() < 1 {
                return Err(RuntimeError::new(
                    "split requires a delimiter argument",
                    line,
                    column,
                ));
            }
            let delimiter_val = eval(args[0].clone(), env)?;
            if let Value::String(delimiter) = delimiter_val {
                let parts: Vec<Value> = string
                    .split(&delimiter)
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                Ok(Value::List(parts))
            } else {
                Err(RuntimeError::new(
                    "split delimiter must be a string",
                    line,
                    column,
                ))
            }
        }
        "replace" => {
            if args.len() < 2 {
                return Err(RuntimeError::new(
                    "replace requires from and to arguments",
                    line,
                    column,
                ));
            }
            let from_val = eval(args[0].clone(), env)?;
            let to_val = eval(args[1].clone(), env)?;
            if let (Value::String(from), Value::String(to)) = (from_val, to_val) {
                Ok(Value::String(string.replace(&from, &to)))
            } else {
                Err(RuntimeError::new(
                    "replace arguments must be strings",
                    line,
                    column,
                ))
            }
        }
        _ => Err(RuntimeError::new(
            format!("{} is not a method of string", method_name).as_str(),
            line,
            column,
        )),
    }
}

/// Valueに応じた builtin メソッドの呼び出し
fn call_builtin_method(
    value: Value,
    method_name: &str,
    args: &[ASTNode],
    caller_ast: &ASTNode,
    env: &mut Env,
    line: usize,
    column: usize,
) -> Result<Value, RuntimeError> {
    match value {
        Value::Number(num) => call_builtin_method_on_number(num, method_name, args, line, column),
        Value::List(list) => {
            call_builtin_method_on_list(list, method_name, args, caller_ast, env, line, column)
        }
        Value::Dict(dict) => {
            call_builtin_method_on_dict(dict, method_name, args, caller_ast, env, line, column)
        }
        Value::String(string) => {
            call_builtin_method_on_string(string, method_name, args, env, line, column)
        }
        _ => Err(RuntimeError::new(
            format!("Method {} is not supported for this type", method_name).as_str(),
            line,
            column,
        )),
    }
}

pub fn builtin_method_call_node(
    method_name: String,
    caller: Box<ASTNode>,
    arguments: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let args = extract_arguments(arguments);
    let value = eval(*caller.clone(), env)?;
    call_builtin_method(value, &method_name, &args, &caller, env, line, column)
}

/// 構造体メソッド呼び出しの本体
pub fn method_call_node(
    method_name: String,
    caller: Box<ASTNode>,
    arguments: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    // 引数リストの取り出し
    let args_vec = match *arguments {
        ASTNode::FunctionCallArgs { args, .. } => args,
        _ => vec![],
    };
    // caller が変数であることを確認し、変数名を取得する
    let caller_name = match *caller {
        ASTNode::Variable { name, .. } => name,
        _ => {
            return Err(RuntimeError::new(
                format!("Unexpected caller: {:?}", caller).as_str(),
                line,
                column,
            ))
        }
    };

    // 環境から caller の変数情報を取得
    let variable_info = env.get(&caller_name, None).ok_or_else(|| {
        RuntimeError::new(
            format!("missing struct: {:?}", caller_name).as_str(),
            line,
            column,
        )
    })?;

    // 構造体情報の取得
    let mut local_env = env.clone();
    let struct_info = match &variable_info.value_type {
        ValueType::StructInstance {
            name: struct_name, ..
        } => local_env.get_struct(struct_name).cloned(),
        ValueType::Struct {
            name: struct_name, ..
        } => local_env.get_struct(struct_name).cloned(),
        _ => {
            return Err(RuntimeError::new(
                format!("missing struct: {:?}", variable_info.value).as_str(),
                line,
                column,
            ))
        }
    };

    let methods = match &struct_info {
        Some(Value::Struct { methods, .. }) => methods,
        _ => {
            return Err(RuntimeError::new(
                format!("failed get methods: {:?}", struct_info).as_str(),
                line,
                column,
            ))
        }
    };

    // 対象のメソッド情報を取得する
    let method_info = methods.get(&method_name).ok_or_else(|| {
        RuntimeError::new(
            format!("call failed method: {:?}", method_name).as_str(),
            line,
            column,
        )
    })?;

    // 変更可能な変数であることの確認
    if variable_info.variable_type == EnvVariableType::Immutable {
        return Err(RuntimeError::new(
            format!("{} is not mutable", caller_name).as_str(),
            line,
            column,
        ));
    }
    if args_vec.len() != method_info.arguments.len() - 1 {
        return Err(RuntimeError::new(
            format!("does not match arguments length: {:?}", args_vec).as_str(),
            line,
            column,
        ));
    }

    // ローカル環境にスコープを追加して、self の設定や引数の割り当てを行う
    local_env.enter_scope(method_name.clone());
    let self_value = variable_info.value.clone();
    let result = local_env.set(
        "self".to_string(),
        self_value.clone(),
        EnvVariableType::Mutable,
        // self の型情報は構造体定義から組み立てる
        match &struct_info {
            Some(Value::Struct {
                name,
                fields,
                methods: _,
            }) => {
                let mut field_types = HashMap::new();
                for (field_name, field_value) in fields {
                    field_types.insert(field_name.to_string(), field_value.value_type());
                }
                ValueType::Struct {
                    name: name.to_string(),
                    fields: field_types,
                    methods: methods.clone(),
                }
            }
            _ => ValueType::Any,
        },
        true,
    );
    if let Err(e) = result {
        return Err(RuntimeError::new(e.as_str(), line, column));
    }
    // struct インスタンスのフィールドもローカル環境にセットする
    if let Value::StructInstance { fields, .. } = self_value {
        for (field_name, field_value) in fields {
            let result = local_env.set(
                field_name.to_string(),
                field_value.clone(),
                EnvVariableType::Mutable,
                field_value.value_type(),
                true,
            );
            if let Err(e) = result {
                return Err(RuntimeError::new(e.as_str(), line, column));
            }
        }
    } else {
        return Err(RuntimeError::new(
            format!("missing struct instance: {:?}", variable_info.value).as_str(),
            line,
            column,
        ));
    }

    // 定義された引数に対して評価して割り当てる
    for (i, define_arg) in method_info.arguments.iter().enumerate() {
        // self はすでにセット済みなのでスキップ
        if let ASTNode::Variable {
            name, value_type, ..
        } = define_arg
        {
            if name == "self" {
                continue;
            }
            let arg_value = eval(args_vec[i - 1].clone(), &mut local_env.clone())?;
            let result = local_env.set(
                name.to_string(),
                arg_value,
                EnvVariableType::Immutable,
                value_type.clone().unwrap_or(ValueType::Any),
                true,
            );
            if let Err(e) = result {
                return Err(RuntimeError::new(e.as_str(), line, column));
            }
        }
    }

    // メソッド本体の評価
    let result = eval(method_info.body.clone().unwrap(), &mut local_env)?;
    // Returnに包まれている場合は中身を取り出す
    let unwrapped_result = match result {
        Value::Return(inner) => *inner,
        other => other,
    };

    // メソッド呼び出し後、self の変更があればグローバル環境に反映する
    if let Some(self_var) = local_env.get(&"self".to_string(), None) {
        if let Value::StructInstance { .. } = self_var.value.clone() {
            let result = local_env.set(
                caller_name.to_string(),
                self_var.value.clone(),
                variable_info.variable_type.clone(),
                variable_info.value_type.clone(),
                false,
            );
            if let Err(e) = result {
                return Err(RuntimeError::new(e.as_str(), line, column));
            }
        }
    }
    env.update_global_env(&local_env);
    env.leave_scope();
    Ok(unwrapped_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtins;
    use crate::evals::evals;
    use crate::parsers::Parser;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_to_string_method_call_node() {
        let mut env = Env::new();
        let input = "1.to_string()".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[0], Value::String("1".to_string()));
    }

    #[test]
    fn test_round_method_call_node() {
        let mut env = Env::new();
        let input = "(1.5).round()".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse();
        let result = eval(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result, Value::Number(2.into()));
    }

    #[test]
    fn test_sqrt_method_call_node() {
        let mut env = Env::new();
        let input = "(2 + 2).sqrt()".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse();
        let result = eval(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result, Value::Number(2.into()));
    }

    #[test]
    fn test_new_method_call_node() {
        let mut env = Env::new();
        let input = r#"
        struct Point {
            x: number,
            y: number,
        }
        impl Point {
          fun new(x: number, y: number): Point {
            return Point { x: x, y: y }
          }
          fun get_x(self): number {
            return self.x
          }
        }
        val mut p = Point{x: 3, y: 2}
        p.get_x()
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env);
        assert_eq!(result.unwrap().last(), Some(&Value::Number(3.into())));
    }
    #[test]
    fn test_push_method_call_node() {
        let mut env = Env::new();

        let input = "val mut xs = []\nxs.push(1)\n".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[1], Value::List(vec![Value::Number(1.into())]));
    }

    #[test]
    fn test_push_method_call_node_with_variable() {
        let mut env = Env::new();
        let input = "val mut xs = [1,2]\nval x = 3\nxs.push(x)\n".to_string();
        let tokens = tokenize(&input);
        let builtin = register_builtins(&mut env);
        let mut parser = Parser::new(tokens, builtin);
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(
            result[2],
            Value::List(vec![
                Value::Number(1.into()),
                Value::Number(2.into()),
                Value::Number(3.into())
            ])
        );
    }
    #[test]
    fn method_chaining_with_round_and_to_string() {
        let mut env = Env::new();
        let input = "fun add(x: number): number {\n return x + 1\n}\n add(1.5).round().to_string()"
            .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[1], Value::String("3".to_string()));
    }

    #[test]
    fn test_list_len_method() {
        let input = r#"
        val xs = [1, 2, 3, 4, 5]
        xs.len()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(5));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_list_pop_method() {
        let input = r#"
        val mut xs = [1, 2, 3]
        xs.pop()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Option(Some(value)) => {
                assert_eq!(*value, Value::Number(Fraction::from(3)));
            }
            _ => panic!("Expected Some(3)"),
        }
    }

    #[test]
    fn test_dict_len_method() {
        let input = r#"
        val d = {: "a" => 1, "b" => 2 :}
        d.len()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();

        println!("AST[0]: {:?}", ast[0]);
        println!("AST[1]: {:?}", ast[1]);

        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        if let Err(e) = &result {
            println!("Dict len error: {:?}", e);
        }
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(2));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_dict_keys_method() {
        let input = r#"
        val d = {: "a" => 1, "b" => 2 :}
        d.keys()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(keys) => {
                assert_eq!(keys.len(), 2);
                // キーの順序は保証されないので、両方のキーが含まれていることを確認
                let key_strings: Vec<String> = keys
                    .iter()
                    .map(|v| {
                        if let Value::String(s) = v {
                            s.clone()
                        } else {
                            panic!("Expected string key")
                        }
                    })
                    .collect();
                assert!(key_strings.contains(&"a".to_string()));
                assert!(key_strings.contains(&"b".to_string()));
            }
            _ => panic!("Expected list of keys"),
        }
    }

    #[test]
    fn test_string_len_method() {
        let input = r#"
        val s = "hello"
        s.len()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(5));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_string_to_uppercase_method() {
        let input = r#"
        val s = "hello"
        s.to_uppercase()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();
        // 最初に変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 次にメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::String(s) => {
                assert_eq!(s, "HELLO");
            }
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_dict_index_assignment() {
        let input = r#"
        val mut d = {: "a" => 1 :}
        d["b"] = 2
        d["a"]
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();

        // 変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 辞書に新しいキーを追加
        eval(ast[1].clone(), &mut env).unwrap();
        // 元のキーの値を取得
        let result = eval(ast[2].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(1));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_dict_index_update() {
        let input = r#"
        val mut d = {: "a" => 1, "b" => 2 :}
        d["a"] = 10
        d["a"]
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();

        // 変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // 既存のキーの値を更新
        eval(ast[1].clone(), &mut env).unwrap();
        // 更新された値を取得
        let result = eval(ast[2].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(10));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_dict_update_method() {
        let input = r#"
        val mut d1 = {: "a" => 1 :}
        val d2 = {: "b" => 2, "c" => 3 :}
        d1.update(d2)
        d1.len()
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();

        // 変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        eval(ast[1].clone(), &mut env).unwrap();
        // updateメソッドを呼び出し
        eval(ast[2].clone(), &mut env).unwrap();
        // 更新後のサイズを確認
        let result = eval(ast[3].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(3));
            }
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_dict_get_or_insert_method() {
        let input = r#"
        val mut d = {: "a" => 1 :}
        d.get_or_insert("b", 42)
        "#;
        let mut env = Env::new();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines().unwrap();

        // 変数を定義
        eval(ast[0].clone(), &mut env).unwrap();
        // get_or_insertメソッドを呼び出し
        let result = eval(ast[1].clone(), &mut env);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Number(n) => {
                assert_eq!(n, Fraction::from(42));
            }
            _ => panic!("Expected number"),
        }
    }
}
