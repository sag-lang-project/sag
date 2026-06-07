use crate::environment::ValueType;
use crate::rc_env::RcEnv;
use crate::rc_value::RcValue;
use fraction::{Fraction, ToPrimitive};
use std::collections::HashMap;
use std::rc::Rc;

pub fn register_rc_builtins(env: &mut RcEnv) -> HashMap<(String, String), ValueType> {
    let mut builtins = HashMap::new();

    // print関数を登録
    env.register_rc_builtin("print".to_string(), rc_print);
    builtins.insert(("global".into(), "print".to_string()), ValueType::Void);

    // len関数を登録
    env.register_rc_builtin("len".to_string(), rc_len);
    builtins.insert(("global".into(), "len".to_string()), ValueType::Number);

    // range関数を登録
    env.register_rc_builtin("range".to_string(), rc_range);
    builtins.insert(
        ("global".into(), "range".to_string()),
        ValueType::List(Box::new(ValueType::Number)),
    );

    builtins
}

fn rc_print(args: Vec<RcValue>) -> RcValue {
    // 引数を表示
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    println!();
    RcValue::Void
}

fn rc_len(args: Vec<RcValue>) -> RcValue {
    if args.len() != 1 {
        panic!("len() takes exactly one argument");
    }

    match &args[0] {
        RcValue::List(list) => RcValue::Number(Fraction::from(list.len())),
        RcValue::String(s) => RcValue::Number(Fraction::from(s.len())),
        RcValue::Dict(dict) => RcValue::Number(Fraction::from(dict.len())),
        _ => panic!("len() requires a list, string, or dictionary"),
    }
}

fn rc_range(args: Vec<RcValue>) -> RcValue {
    let (start, end, step) = match args.len() {
        1 => {
            let end = match &args[0] {
                RcValue::Number(n) => n.to_i32().unwrap_or(0),
                _ => panic!("range() requires numeric arguments"),
            };
            (0, end, 1)
        }
        2 => {
            let start = match &args[0] {
                RcValue::Number(n) => n.to_i32().unwrap_or(0),
                _ => panic!("range() requires numeric arguments"),
            };
            let end = match &args[1] {
                RcValue::Number(n) => n.to_i32().unwrap_or(0),
                _ => panic!("range() requires numeric arguments"),
            };
            (start, end, 1)
        }
        3 => {
            let start = match &args[0] {
                RcValue::Number(n) => n.to_i32().unwrap_or(0),
                _ => panic!("range() requires numeric arguments"),
            };
            let end = match &args[1] {
                RcValue::Number(n) => n.to_i32().unwrap_or(0),
                _ => panic!("range() requires numeric arguments"),
            };
            let step = match &args[2] {
                RcValue::Number(n) => n.to_i32().unwrap_or(1),
                _ => panic!("range() requires numeric arguments"),
            };
            (start, end, step)
        }
        _ => panic!("range() takes 1-3 arguments"),
    };

    if step == 0 {
        panic!("range() step cannot be zero");
    }

    let mut result = Vec::new();
    let mut i = start;

    if step > 0 {
        while i < end {
            result.push(RcValue::Number(Fraction::from(i)));
            i += step;
        }
    } else {
        while i > end {
            result.push(RcValue::Number(Fraction::from(i)));
            i += step;
        }
    }

    RcValue::List(Rc::new(result))
}
