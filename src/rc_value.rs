use crate::ast::ASTNode;
use crate::environment::Env;
use crate::environment::{MethodInfo, ValueType};
use crate::value::Value;
use fraction::Fraction;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// Rc版のValue型。参照カウントを使用してクローン操作のパフォーマンスを向上させる。
#[derive(Debug, Clone, PartialEq)]
pub enum RcValue {
    Option(Option<Rc<RcValue>>),
    Result(Result<Rc<RcValue>, Rc<RcValue>>),
    Number(Fraction),
    String(Rc<String>),
    Bool(bool),
    Void,
    List(Rc<Vec<RcValue>>),
    Dict(Rc<HashMap<String, RcValue>>),
    Function,
    Return(Rc<RcValue>),
    Break,
    Continue,
    Struct {
        name: Rc<String>,
        fields: Rc<HashMap<String, RcValue>>,
        methods: Rc<HashMap<String, MethodInfo>>,
    },
    StructInstance {
        name: Rc<String>,
        fields: Rc<HashMap<String, RcValue>>,
    },
    StructField {
        value_type: ValueType,
        is_public: bool,
    },
    Impl {
        base_struct: ValueType,
        methods: Rc<HashMap<String, MethodInfo>>,
    },
    Lambda {
        arguments: Rc<Vec<ASTNode>>,
        body: Rc<ASTNode>,
        env: Rc<Env>,
    },
}

impl RcValue {
    /// 通常のValueからRcValueに変換する（最適化版）
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Number(n) => RcValue::Number(n.clone()),
            Value::String(s) => RcValue::new_string(s.clone()),
            Value::Bool(b) => RcValue::Bool(*b),
            Value::Void => RcValue::Void,
            Value::Function => RcValue::Function,
            Value::Break => RcValue::Break,
            Value::Continue => RcValue::Continue,
            Value::Option(opt) => {
                RcValue::Option(opt.as_ref().map(|v| Rc::new(RcValue::from_value(v))))
            }
            Value::Result(res) => match res {
                Ok(v) => RcValue::Result(Ok(Rc::new(RcValue::from_value(v)))),
                Err(v) => RcValue::Result(Err(Rc::new(RcValue::from_value(v)))),
            },
            Value::List(list) => {
                // リストの要素を一度だけ変換
                let rc_list = list
                    .iter()
                    .map(|v| RcValue::from_value(v))
                    .collect::<Vec<_>>();
                RcValue::List(Rc::new(rc_list))
            }
            Value::Dict(dict) => {
                // 辞書の要素を一度だけ変換
                let mut rc_dict = HashMap::new();
                for (k, v) in dict {
                    rc_dict.insert(k.clone(), RcValue::from_value(v));
                }
                RcValue::Dict(Rc::new(rc_dict))
            }
            Value::Return(v) => RcValue::Return(Rc::new(RcValue::from_value(v))),
            Value::Struct {
                name,
                fields,
                methods,
            } => {
                let mut rc_fields = HashMap::new();
                for (k, v) in fields {
                    rc_fields.insert(k.clone(), RcValue::from_value(v));
                }
                RcValue::Struct {
                    name: Rc::new(name.clone()),
                    fields: Rc::new(rc_fields),
                    methods: Rc::new(methods.clone()),
                }
            }
            Value::StructInstance { name, fields } => {
                let mut rc_fields = HashMap::new();
                for (k, v) in fields {
                    rc_fields.insert(k.clone(), RcValue::from_value(v));
                }
                RcValue::StructInstance {
                    name: Rc::new(name.clone()),
                    fields: Rc::new(rc_fields),
                }
            }
            Value::StructField {
                value_type,
                is_public,
            } => RcValue::StructField {
                value_type: value_type.clone(),
                is_public: *is_public,
            },
            Value::Impl {
                base_struct,
                methods,
            } => RcValue::Impl {
                base_struct: base_struct.clone(),
                methods: Rc::new(methods.clone()),
            },
            Value::Lambda {
                arguments,
                body,
                env,
            } => RcValue::Lambda {
                arguments: Rc::new(arguments.clone()),
                body: Rc::new(body.as_ref().clone()),
                env: Rc::new(env.clone()),
            },
        }
    }

    /// RcValueから通常のValueに変換する（最適化版）
    pub fn to_value(&self) -> Value {
        match self {
            RcValue::Number(n) => Value::Number(n.clone()),
            RcValue::String(s) => Value::String(s.to_string()),
            RcValue::Bool(b) => Value::Bool(*b),
            RcValue::Void => Value::Void,
            RcValue::Function => Value::Function,
            RcValue::Break => Value::Break,
            RcValue::Continue => Value::Continue,
            RcValue::Option(opt) => Value::Option(opt.as_ref().map(|v| Box::new(v.to_value()))),
            RcValue::Result(res) => match res {
                Ok(v) => Value::Result(Ok(Box::new(v.to_value()))),
                Err(v) => Value::Result(Err(Box::new(v.to_value()))),
            },
            RcValue::List(list) => {
                let value_list = list.iter().map(|v| v.to_value()).collect();
                Value::List(value_list)
            }
            RcValue::Dict(dict) => {
                let mut value_dict = HashMap::new();
                for (k, v) in dict.iter() {
                    value_dict.insert(k.clone(), v.to_value());
                }
                Value::Dict(value_dict)
            }
            RcValue::Return(v) => Value::Return(Box::new(v.to_value())),
            RcValue::Struct {
                name,
                fields,
                methods,
            } => {
                let mut value_fields = HashMap::new();
                for (k, v) in fields.iter() {
                    value_fields.insert(k.clone(), v.to_value());
                }
                Value::Struct {
                    name: name.to_string(),
                    fields: value_fields,
                    methods: methods.as_ref().clone(),
                }
            }
            RcValue::StructInstance { name, fields } => {
                let mut value_fields = HashMap::new();
                for (k, v) in fields.iter() {
                    value_fields.insert(k.clone(), v.to_value());
                }
                Value::StructInstance {
                    name: name.to_string(),
                    fields: value_fields,
                }
            }
            RcValue::StructField {
                value_type,
                is_public,
            } => Value::StructField {
                value_type: value_type.clone(),
                is_public: *is_public,
            },
            RcValue::Impl {
                base_struct,
                methods,
            } => Value::Impl {
                base_struct: base_struct.clone(),
                methods: methods.as_ref().clone(),
            },
            RcValue::Lambda {
                arguments,
                body,
                env,
            } => Value::Lambda {
                arguments: arguments.as_ref().clone(),
                body: Box::new(body.as_ref().clone()),
                env: env.as_ref().clone(),
            },
        }
    }

    /// 値の型を取得する
    pub fn value_type(&self) -> ValueType {
        match self {
            RcValue::Number(_) => ValueType::Number,
            RcValue::String(_) => ValueType::String,
            RcValue::Bool(_) => ValueType::Bool,
            RcValue::Void => ValueType::Void,
            RcValue::Option(v) => {
                if v.is_none() {
                    ValueType::OptionType(Box::new(ValueType::Any))
                } else {
                    ValueType::OptionType(Box::new(v.as_ref().unwrap().value_type()))
                }
            }
            RcValue::Result(v) => {
                if v.is_ok() {
                    ValueType::ResultType {
                        success: Box::new(v.as_ref().unwrap().value_type()),
                        failure: Box::new(ValueType::Void),
                    }
                } else {
                    ValueType::ResultType {
                        success: Box::new(ValueType::Void),
                        failure: Box::new(v.as_ref().unwrap().value_type()),
                    }
                }
            }
            RcValue::Impl { base_struct, .. } => ValueType::Impl {
                base_struct: Box::new(base_struct.clone()),
                methods: HashMap::new(),
            },
            RcValue::StructInstance { name, fields } => {
                let mut field_types = HashMap::new();
                for (field_name, field_value) in fields.iter() {
                    field_types.insert(field_name.clone(), field_value.value_type());
                }
                ValueType::StructInstance {
                    name: name.to_string(),
                    fields: field_types,
                }
            }
            RcValue::StructField { value_type, .. } => value_type.clone(),
            RcValue::Struct { name, fields, .. } => {
                let field_types = fields
                    .iter()
                    .map(|(name, field)| {
                        if let RcValue::StructField { value_type, .. } = field {
                            (name.clone(), value_type.clone())
                        } else {
                            panic!("invalid struct field")
                        }
                    })
                    .collect::<HashMap<_, _>>();
                ValueType::Struct {
                    name: name.to_string(),
                    fields: field_types,
                    methods: HashMap::new(),
                }
            }
            RcValue::List(values) => {
                if values.is_empty() {
                    ValueType::List(Box::new(ValueType::Any))
                } else {
                    let mut value_type = values[0].value_type();
                    for value in values.iter().skip(1) {
                        if value.value_type() != value_type {
                            value_type = ValueType::Any;
                            break;
                        }
                    }
                    ValueType::List(Box::new(value_type))
                }
            }
            RcValue::Dict(dict) => {
                let mut value_type = ValueType::Any;
                for key in dict.keys() {
                    value_type = dict.get(key).unwrap().value_type();
                    break;
                }
                ValueType::Dict(Box::new(value_type))
            }
            RcValue::Function => ValueType::Function,
            RcValue::Return(value) => value.value_type(),
            RcValue::Break => ValueType::Void,
            RcValue::Continue => ValueType::Void,
            RcValue::Lambda { .. } => ValueType::Lambda,
        }
    }

    // ヘルパーメソッド
    pub fn new_string(s: String) -> Self {
        RcValue::String(Rc::new(s))
    }

    pub fn new_list(list: Vec<RcValue>) -> Self {
        RcValue::List(Rc::new(list))
    }

    pub fn new_dict(dict: HashMap<String, RcValue>) -> Self {
        RcValue::Dict(Rc::new(dict))
    }

    pub fn new_struct(
        name: String,
        fields: HashMap<String, RcValue>,
        methods: HashMap<String, MethodInfo>,
    ) -> Self {
        RcValue::Struct {
            name: Rc::new(name),
            fields: Rc::new(fields),
            methods: Rc::new(methods),
        }
    }

    pub fn new_struct_instance(name: String, fields: HashMap<String, RcValue>) -> Self {
        RcValue::StructInstance {
            name: Rc::new(name),
            fields: Rc::new(fields),
        }
    }

    pub fn new_lambda(arguments: Vec<ASTNode>, body: ASTNode, env: Env) -> Self {
        RcValue::Lambda {
            arguments: Rc::new(arguments),
            body: Rc::new(body),
            env: Rc::new(env),
        }
    }

    pub fn new_option(value: Option<RcValue>) -> Self {
        RcValue::Option(value.map(|v| Rc::new(v)))
    }

    pub fn new_result_ok(value: RcValue) -> Self {
        RcValue::Result(Ok(Rc::new(value)))
    }

    pub fn new_result_err(value: RcValue) -> Self {
        RcValue::Result(Err(Rc::new(value)))
    }

    pub fn new_return(value: RcValue) -> Self {
        RcValue::Return(Rc::new(value))
    }

    // Copy-on-Write操作
    pub fn list_push(&self, value: RcValue) -> Self {
        match self {
            RcValue::List(list) => {
                // 参照カウントが1の場合は直接変更（CoW）
                if Rc::strong_count(list) == 1 {
                    // 安全に変更するには、Rcの可変参照が必要
                    // しかし、selfは不変参照なので、新しいリストを作成する
                    let mut new_list = list.as_ref().clone();
                    new_list.push(value);
                    RcValue::List(Rc::new(new_list))
                } else {
                    // 参照が共有されている場合はコピーして変更
                    let mut new_list = list.as_ref().clone();
                    new_list.push(value);
                    RcValue::List(Rc::new(new_list))
                }
            }
            _ => panic!("Expected a list"),
        }
    }

    pub fn dict_insert(&self, key: String, value: RcValue) -> Self {
        match self {
            RcValue::Dict(dict) => {
                // 参照カウントが1の場合は直接変更（CoW）
                if Rc::strong_count(dict) == 1 {
                    // 安全に変更するには、Rcの可変参照が必要
                    // しかし、selfは不変参照なので、新しい辞書を作成する
                    let mut new_dict = dict.as_ref().clone();
                    new_dict.insert(key, value);
                    RcValue::Dict(Rc::new(new_dict))
                } else {
                    // 参照が共有されている場合はコピーして変更
                    let mut new_dict = dict.as_ref().clone();
                    new_dict.insert(key, value);
                    RcValue::Dict(Rc::new(new_dict))
                }
            }
            _ => panic!("Expected a dictionary"),
        }
    }

    pub fn struct_update_field(&self, field_name: &str, value: RcValue) -> Self {
        match self {
            RcValue::StructInstance { name, fields } => {
                // 参照カウントが1の場合は直接変更（CoW）
                if Rc::strong_count(fields) == 1 {
                    // 安全に変更するには、Rcの可変参照が必要
                    // しかし、selfは不変参照なので、新しいフィールドマップを作成する
                    let mut new_fields = fields.as_ref().clone();
                    new_fields.insert(field_name.to_string(), value);
                    RcValue::StructInstance {
                        name: name.clone(),
                        fields: Rc::new(new_fields),
                    }
                } else {
                    // 参照が共有されている場合はコピーして変更
                    let mut new_fields = fields.as_ref().clone();
                    new_fields.insert(field_name.to_string(), value);
                    RcValue::StructInstance {
                        name: name.clone(),
                        fields: Rc::new(new_fields),
                    }
                }
            }
            _ => panic!("Expected a struct instance"),
        }
    }

    // リストの要素にアクセス
    pub fn list_get(&self, index: usize) -> Option<RcValue> {
        match self {
            RcValue::List(list) => {
                if index < list.len() {
                    Some(list[index].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // 辞書の要素にアクセス
    pub fn dict_get(&self, key: &str) -> Option<RcValue> {
        match self {
            RcValue::Dict(dict) => dict.get(key).cloned(),
            _ => None,
        }
    }
}

impl fmt::Display for RcValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RcValue::Number(value) => write!(f, "{}", value),
            RcValue::String(s) => write!(f, "{}", s),
            RcValue::Bool(b) => write!(f, "{}", b),
            RcValue::Void => write!(f, "Void"),
            RcValue::Function => write!(f, "Function"),
            RcValue::Lambda { .. } => write!(f, "Lambda"),
            RcValue::Return(value) => write!(f, "{}", value),
            RcValue::Break => write!(f, "Break"),
            RcValue::Continue => write!(f, "Continue"),
            RcValue::Option(option) => match option {
                Some(value) => write!(f, "{}", value),
                None => write!(f, "None"),
            },
            RcValue::Result(result) => match result {
                Ok(value) => write!(f, "Suc({})", value),
                Err(value) => write!(f, "Fail({})", value),
            },
            RcValue::Impl {
                base_struct,
                methods,
            } => {
                let mut result = String::new();
                result.push_str(&format!("Impl {{\n"));
                result.push_str(&format!("    base_struct: {:?},\n", base_struct));
                for method in methods.iter() {
                    result.push_str(&format!("    method: {:?},\n", method));
                }
                result.push_str("}");
                write!(f, "{}", result)
            }
            RcValue::StructInstance { name, fields } => {
                let mut result = String::new();
                result.push_str(&format!("{} {{\n", name));
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        result.push_str(",\n");
                    }
                    result.push_str(&format!("    {}: {}", field.0, field.1));
                }
                result.push_str("\n}");
                write!(f, "{}", result)
            }
            RcValue::Struct { name, fields, .. } => {
                let mut result = String::new();
                result.push_str(&format!("{} {{", name));
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("{:?}", field));
                }
                result.push_str("}");
                write!(f, "{}", result)
            }
            RcValue::StructField { value_type, .. } => write!(f, "{:?}", value_type.clone()),
            RcValue::List(list) => {
                let mut result = String::new();
                for (i, value) in list.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("{}", value));
                }
                write!(f, "[{}]", result)
            }
            RcValue::Dict(dict) => {
                let mut result = String::new();
                for (i, (key, value)) in dict.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("{}: {}", key, value));
                }
                write!(f, "{{:{}:}}", result)
            }
        }
    }
}
