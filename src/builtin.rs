use crate::environment::Env;
use crate::environment::ValueType;
use crate::value::Value;
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
pub fn register_builtins(env: &mut Env) -> HashMap<(String, String), ValueType> {
    let mut builtins = HashMap::new();
    env.register_builtin("print".to_string(), |args: Vec<Value>| {
        for arg in args {
            print!("{} ", arg);
        }
        println!();
        Value::Void
    });
    builtins.insert(("global".into(), "print".to_string()), ValueType::Void);

    env.register_builtin("len".to_string(), |args: Vec<Value>| {
        if args.len() != 1 {
            panic!("len function takes exactly one argument");
        }
        match &args[0] {
            Value::List(l) => Value::Int(l.len() as i64),
            Value::String(s) => Value::Int(s.len() as i64),
            _ => panic!("len function takes a list as an argument"),
        }
    });
    builtins.insert(("global".into(), "len".to_string()), ValueType::Number);

    env.register_builtin("range".to_string(), |args: Vec<Value>| {
        if let [start, end] = args.as_slice() {
            if let (Some(start), Some(end)) = (start.to_i64_if_integer(), end.to_i64_if_integer()) {
                Value::List((start..end).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else if let [end] = args.as_slice() {
            if let Some(end) = end.to_i64_if_integer() {
                Value::List((0..end).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else if let [start, end, step] = args.as_slice() {
            if let (Some(start), Some(end), Some(step)) = (
                start.to_i64_if_integer(),
                end.to_i64_if_integer(),
                step.to_i64_if_integer(),
            ) {
                if step <= 0 {
                    panic!("range function step must be positive");
                }
                Value::List((start..end).step_by(step as usize).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else {
            panic!("range function takes 1, 2 or 3 arguments")
        }
    });
    builtins.insert(("global".into(), "range".to_string()), ValueType::Number);
    builtins
}

#[cfg(target_arch = "wasm32")]
pub fn register_builtins(env: &mut Env) -> HashMap<(String, String), ValueType> {
    use crate::wasm::CONSOLE_OUTPUT;

    let mut builtins = HashMap::new();
    env.register_builtin("print".to_string(), |args: Vec<Value>| {
        let output = args
            .iter()
            .map(|arg| format!("{}", arg))
            .collect::<Vec<_>>()
            .join(" ");

        CONSOLE_OUTPUT.with(|console| {
            let mut console = console.borrow_mut();
            if !console.is_empty() {
                console.push('\n');
            }
            console.push_str(&output);
        });

        Value::Void
    });
    builtins.insert(("global".into(), "print".to_string()), ValueType::Void);

    env.register_builtin("len".to_string(), |args: Vec<Value>| {
        if args.len() != 1 {
            panic!("len function takes exactly one argument");
        }
        match &args[0] {
            Value::List(l) => Value::Int(l.len() as i64),
            Value::String(s) => Value::Int(s.len() as i64),
            _ => panic!("len function takes a list as an argument"),
        }
    });
    builtins.insert(("global".into(), "len".to_string()), ValueType::Number);

    env.register_builtin("range".to_string(), |args: Vec<Value>| {
        if let [start, end] = args.as_slice() {
            if let (Some(start), Some(end)) = (start.to_i64_if_integer(), end.to_i64_if_integer()) {
                Value::List((start..end).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else if let [end] = args.as_slice() {
            if let Some(end) = end.to_i64_if_integer() {
                Value::List((0..end).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else if let [start, end, step] = args.as_slice() {
            if let (Some(start), Some(end), Some(step)) = (
                start.to_i64_if_integer(),
                end.to_i64_if_integer(),
                step.to_i64_if_integer(),
            ) {
                if step <= 0 {
                    panic!("range function step must be positive");
                }
                Value::List((start..end).step_by(step as usize).map(Value::Int).collect())
            } else {
                panic!("range function takes integer arguments")
            }
        } else {
            panic!("range function takes 1, 2 or 3 arguments")
        }
    });
    builtins.insert(("global".into(), "range".to_string()), ValueType::Number);
    builtins
}
