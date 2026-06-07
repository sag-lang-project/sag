use crate::ast::ASTNode;
use crate::environment::Env;
use crate::environment::{MethodInfo, ValueType};
use fraction::Fraction;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Int(i64),
    Number(Fraction),
    String(String),
    Bool(bool),
    Void,
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
    Function,
    Return(Box<Value>),
    Break,
    Continue,
    Struct {
        name: String,
        fields: HashMap<String, Value>, // field_name: value
        methods: HashMap<String, MethodInfo>,
    },
    StructInstance {
        name: String,
        fields: HashMap<String, Value>,
    },
    StructField {
        value_type: ValueType,
        is_public: bool,
    },
    Impl {
        base_struct: ValueType,
        methods: HashMap<String, MethodInfo>,
    },
    Lambda {
        arguments: Vec<ASTNode>,
        body: Box<ASTNode>,
        env: Env,
    },
}

impl Value {
    pub fn from_fraction(value: Fraction) -> Self {
        if value.denom() == Some(&1) {
            Value::Int(*value.numer().unwrap() as i64)
        } else {
            Value::Number(value)
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Int(_) => ValueType::Number,
            Value::Number(_) => ValueType::Number,
            Value::String(_) => ValueType::String,
            Value::Bool(_) => ValueType::Bool,
            Value::Void => ValueType::Void,
            Value::Option(v) => {
                if v.is_none() {
                    ValueType::OptionType(Box::new(ValueType::Any))
                } else {
                    ValueType::OptionType(Box::new(v.as_ref().unwrap().value_type()))
                }
            }
            Value::Result(v) => {
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
            Value::Impl {
                base_struct,
                methods,
            } => ValueType::Impl {
                base_struct: Box::new(base_struct.clone()),
                methods: methods.clone(),
            },
            Value::StructInstance { name, fields } => {
                let mut field_types = HashMap::new();
                for (field_name, field_value) in fields.iter() {
                    field_types.insert(field_name.clone(), field_value.value_type());
                }
                ValueType::StructInstance {
                    name: name.clone(),
                    fields: field_types,
                }
            }
            Value::StructField { value_type, .. } => value_type.clone(),
            Value::Struct {
                name,
                fields,
                methods,
            } => {
                let field_types = fields
                    .iter()
                    .map(|(name, field)| {
                        if let Value::StructField {
                            value_type,
                            is_public: _,
                        } = field
                        {
                            (name.clone(), value_type.clone())
                        } else {
                            panic!("invalid struct field")
                        }
                    })
                    .collect::<HashMap<_, _>>();
                ValueType::Struct {
                    name: name.clone(),
                    fields: field_types.clone(),
                    methods: methods.clone(),
                }
            }
            Value::List(values) => {
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
            Value::Dict(dict) => {
                let mut value_type = ValueType::Any;
                for key in dict.keys() {
                    value_type = dict.get(key).unwrap().value_type();
                    break;
                }
                ValueType::Dict(Box::new(value_type))
            }
            Value::Function => ValueType::Function,
            Value::Return(value) => {
                if let Value::Void = **value {
                    ValueType::Void
                } else {
                    value.value_type()
                }
            }
            Value::Break => ValueType::Void,
            Value::Continue => ValueType::Void,
            Value::Lambda { .. } => ValueType::Lambda,
        }
    }
    pub fn to_number(&self) -> Fraction {
        match self {
            Value::Int(value) => Fraction::from(*value),
            Value::Number(value) => value.clone(),
            _ => panic!("expected number"),
        }
    }

    pub fn to_i64_if_integer(&self) -> Option<i64> {
        match self {
            Value::Int(value) => Some(*value),
            Value::Number(value) if value.denom() == Some(&1) => Some(*value.numer().unwrap() as i64),
            _ => None,
        }
    }
    pub fn to_str(&self) -> String {
        match self {
            Value::String(value) => value.clone(),
            _ => panic!("expected string"),
        }
    }
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(value) => value.clone(),
            _ => panic!("expected bool"),
        }
    }
    pub fn to_list(&self) -> Vec<Value> {
        match self {
            Value::List(value) => value.clone(),
            _ => panic!("expected list"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(value) => write!(f, "{}", value),
            Value::Number(value) => write!(f, "{}", value),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Void => write!(f, "Void"),
            Value::Function => write!(f, "Function"),
            Value::Lambda { .. } => write!(f, "Lambda"),
            Value::Return(value) => write!(f, "{}", value),
            Value::Break => write!(f, "Break"),
            Value::Continue => write!(f, "Continue"),
            Value::Option(option) => match option {
                Some(value) => write!(f, "{}", value),
                None => write!(f, "None"),
            },
            Value::Result(result) => match result {
                Ok(value) => write!(f, "Suc({})", value),
                Err(value) => write!(f, "Fail({})", value),
            },
            Value::Impl {
                base_struct,
                methods,
            } => {
                let mut result = String::new();
                result.push_str(&format!("Impl {{\n"));
                result.push_str(&format!("    base_struct: {:?},\n", base_struct));
                for method in methods {
                    result.push_str(&format!("    method: {:?},\n", method));
                }
                result.push_str("}");
                write!(f, "{}", result)
            }
            Value::StructInstance { name, fields } => {
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
            Value::Struct { name, fields, .. } => {
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
            Value::StructField { value_type, .. } => write!(f, "{:?}", value_type.clone()),
            Value::List(list) => {
                let mut result = String::new();
                for (i, value) in list.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("{}", value));
                }
                write!(f, "[{}]", result)
            }
            Value::Dict(dict) => {
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
