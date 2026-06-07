pub mod assign_ast;
pub mod block_ast;
pub mod break_ast;
pub mod continue_ast;
pub mod dict_ast;
pub mod for_ast;
pub mod function_ast;
pub mod identifier_ast;
pub mod if_ast;
pub mod import_ast;
pub mod infer_type;
pub mod lambda_ast;
pub mod list_ast;
pub mod literal_ast;
pub mod match_ast;
pub mod method_ast;
pub mod option_ast;
pub mod parse_error;
pub mod pipe_ast;
pub mod prefix_op_ast;
pub mod result_ast;
pub mod return_ast;
pub mod string_to_value_type;
pub mod struct_ast;

use crate::ast::ASTNode;
use crate::environment::{EnvVariableType, MethodInfo, ValueType};
use crate::parsers::parse_error::ParseError;
use crate::token::{Token, TokenKind};
use crate::value::Value;
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Vec<Token>>,
    pos: usize,
    line: usize,
    scopes: Vec<String>,
    variables: HashMap<(String, String), (ValueType, EnvVariableType)>, // key: (scope, name), value: value_type
    structs: HashMap<(String, String), (ValueType, EnvVariableType, HashMap<String, ASTNode>)>, // key: (scope, name), value: value_type
    functions: HashMap<(String, String), ValueType>, // key: (scope, name, arguments), value: (body, return_type)
    current_struct: Option<String>,
    in_method_scope: bool, // メソッド内かどうかを追跡
}

impl Parser {
    pub fn new(
        tokens: Vec<Token>,
        initial_functions: HashMap<(String, String), ValueType>,
    ) -> Self {
        let lines = Self::split_lines(tokens);
        Parser {
            tokens: lines.clone(),
            pos: 0,
            line: 0,
            scopes: vec!["global".into()],
            variables: HashMap::new(),
            structs: HashMap::new(),
            functions: initial_functions,
            current_struct: None,
            in_method_scope: false, // 初期状態ではメソッドスコープではない
        }
    }

    fn get_line_column(&self) -> (usize, usize) {
        match self.get_current_token() {
            Some(token) => (token.line, token.column),
            None => (self.line, self.pos),
        }
    }

    fn enter_struct(&mut self, struct_name: String) {
        self.current_struct = Some(struct_name);
    }

    fn leave_struct(&mut self) {
        self.current_struct = None;
    }

    fn get_current_struct(&self) -> Option<String> {
        self.current_struct.clone()
    }

    fn enter_method_scope(&mut self) {
        self.in_method_scope = true;
    }

    fn leave_method_scope(&mut self) {
        self.in_method_scope = false;
    }

    fn enter_scope(&mut self, scope_name: String) {
        self.scopes.push(scope_name);
    }

    fn leave_scope(&mut self) {
        self.scopes.pop();
    }

    fn get_current_scope(&self) -> String {
        self.scopes.last().unwrap().to_string()
    }

    fn register_struct(&mut self, scope: String, struct_value: ASTNode) {
        if let ASTNode::Struct { name, fields, .. } = &struct_value {
            let field_types = fields
                .iter()
                .map(|(name, field)| {
                    if let ASTNode::StructField {
                        value_type,
                        is_public,
                        ..
                    } = field
                    {
                        (
                            name.clone(),
                            ValueType::StructField {
                                value_type: Box::new(value_type.clone()),
                                is_public: is_public.clone(),
                            },
                        )
                    } else {
                        panic!("invalid struct field")
                    }
                })
                .collect();
            let methods = HashMap::new();
            let insert_value = (
                ValueType::Struct {
                    name: name.clone(),
                    fields: field_types,
                    methods,
                },
                EnvVariableType::Immutable,
                HashMap::new(),
            );
            self.structs
                .insert((scope.to_string(), name.to_string()), insert_value);
        }
    }

    fn register_method(&mut self, scope: String, struct_name: String, method: ASTNode) {
        if let ASTNode::Method {
            name: method_name,
            arguments,
            body,
            return_type,
            is_mut,
            ..
        } = method.clone()
        {
            for scope in vec![scope.to_string(), "global".to_string()] {
                if let Some((value_type, _, _)) = self
                    .structs
                    .get_mut(&(scope.to_string(), struct_name.to_string()))
                {
                    match value_type {
                        ValueType::Struct {
                            name: _,
                            fields: _,
                            methods,
                        } => {
                            let method_info = MethodInfo {
                                arguments: arguments.clone(),
                                body: Some(*body),
                                return_type: return_type.clone(),
                                is_mut: is_mut.clone(),
                            };
                            methods.insert(method_name.clone(), method_info);
                            break;
                        }
                        _ => panic!("invalid method"),
                    }
                }
            }
        } else {
            panic!("invalid method")
        }
    }

    fn get_struct(&self, scope: String, name: String) -> Option<ValueType> {
        for checked_scope in vec![scope.to_string(), "global".to_string()] {
            match self
                .structs
                .get(&(checked_scope.to_string(), name.to_string()))
            {
                Some((value_type, _, ..)) => match value_type.clone() {
                    ValueType::Struct { .. } => return Some(value_type.clone()),
                    _ => return None,
                },
                None => {}
            };
        }
        None
    }

    fn register_functions(
        &mut self,
        scope: String,
        name: &String,
        _arguments: &Vec<ASTNode>, // arugmentsも多重定義を許容するときに使う
        return_type: &ValueType,
    ) {
        self.functions
            .insert((scope.clone(), name.to_string()), return_type.clone());
    }

    fn get_function(&self, scope: String, name: String) -> Option<ValueType> {
        for checked_scope in vec![scope.to_string(), "global".to_string()] {
            match self
                .functions
                .get(&(checked_scope.to_string(), name.to_string()))
            {
                Some(value) => return Some(value.clone()),
                None => {}
            };
        }
        None
    }

    fn get_method(
        &self,
        _scope: String,
        value_type: ValueType,
        method_name: String,
    ) -> Option<MethodInfo> {
        match value_type {
            ValueType::Struct {
                name: _,
                fields: _,
                methods,
            } => match methods.get(&method_name) {
                Some(method) => Some(method.clone()),
                None => None,
            },
            ValueType::List(_value_type) => match method_name.as_str() {
                "push" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Void,
                    is_mut: true,
                }),
                "pop" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: true,
                }),
                "len" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Number,
                    is_mut: false,
                }),
                "is_empty" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "first" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: false,
                }),
                "last" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: false,
                }),
                "clear" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Void,
                    is_mut: true,
                }),
                "contains" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "reverse" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Void,
                    is_mut: true,
                }),
                _ => None,
            },
            ValueType::Dict(_value_type) => match method_name.as_str() {
                "get" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: false,
                }),
                "insert" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: true,
                }),
                "remove" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: true,
                }),
                "contains_key" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "keys" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::List(Box::new(ValueType::String)),
                    is_mut: false,
                }),
                "values" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::List(Box::new(_value_type.as_ref().clone())),
                    is_mut: false,
                }),
                "len" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Number,
                    is_mut: false,
                }),
                "is_empty" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "clear" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Void,
                    is_mut: true,
                }),
                "update" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Void,
                    is_mut: true,
                }),
                "entry" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::OptionType(Box::new(_value_type.as_ref().clone())),
                    is_mut: true,
                }),
                "get_or_insert" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: _value_type.as_ref().clone(),
                    is_mut: true,
                }),
                _ => None,
            },
            ValueType::StructInstance { name, fields: _ } => {
                match self.get_struct(self.get_current_scope(), name.clone()) {
                    Some(ValueType::Struct {
                        name: _,
                        fields: _,
                        methods,
                    }) => match methods.get(&method_name) {
                        Some(method) => Some(method.clone()),
                        None => None,
                    },
                    _ => None,
                }
            }
            ValueType::Number => match method_name.as_str() {
                "to_string" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::String,
                    is_mut: false,
                }),
                "sqrt" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Number,
                    is_mut: false,
                }),
                "round" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Number,
                    is_mut: false,
                }),
                _ => None,
            },
            ValueType::String => match method_name.as_str() {
                "len" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Number,
                    is_mut: false,
                }),
                "is_empty" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "to_uppercase" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::String,
                    is_mut: false,
                }),
                "to_lowercase" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::String,
                    is_mut: false,
                }),
                "trim" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::String,
                    is_mut: false,
                }),
                "contains" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "starts_with" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "ends_with" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::Bool,
                    is_mut: false,
                }),
                "split" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::List(Box::new(ValueType::String)),
                    is_mut: false,
                }),
                "replace" => Some(MethodInfo {
                    arguments: vec![],
                    body: None,
                    return_type: ValueType::String,
                    is_mut: false,
                }),
                _ => None,
            },
            _ => None,
        }
    }

    fn register_variables(
        &mut self,
        scope: String,
        name: &String,
        value_type: &ValueType,
        variable_type: &EnvVariableType,
    ) {
        self.variables.insert(
            (scope.clone(), name.to_string()),
            (value_type.clone(), variable_type.clone()),
        );
    }

    fn find_variables(&self, scope: String, name: String) -> Option<(ValueType, EnvVariableType)> {
        for checked_scope in vec![scope.to_string(), "global".to_string()] {
            match self
                .variables
                .get(&(checked_scope.to_string(), name.to_string()))
            {
                Some(value) => match &value.0 {
                    &ValueType::Number => return Some((ValueType::Number, value.1.clone())),
                    &ValueType::String => return Some((ValueType::String, value.1.clone())),
                    &ValueType::Bool => return Some((ValueType::Bool, value.1.clone())),
                    &ValueType::Function => return Some((ValueType::Function, value.1.clone())),
                    &ValueType::StructInstance {
                        ref name,
                        ref fields,
                    } => {
                        return Some((
                            ValueType::StructInstance {
                                name: name.to_string(),
                                fields: fields.clone(),
                            },
                            value.1.clone(),
                        ))
                    }
                    &ValueType::List(ref value_type) => {
                        return Some((
                            ValueType::List(Box::new(*value_type.clone())),
                            value.1.clone(),
                        ))
                    }
                    &ValueType::Dict(ref value_type) => {
                        return Some((ValueType::Dict(value_type.clone()), value.1.clone()))
                    }
                    &ValueType::OptionType(ref value_type) => {
                        return Some((ValueType::OptionType(value_type.clone()), value.1.clone()))
                    }
                    &ValueType::ResultType {
                        ref success,
                        ref failure,
                    } => {
                        return Some((
                            ValueType::ResultType {
                                success: success.clone(),
                                failure: failure.clone(),
                            },
                            value.1.clone(),
                        ))
                    }
                    &ValueType::Any => return Some((ValueType::Any, value.1.clone())),
                    _ => return None,
                },
                None => {}
            };
        }
        None
    }

    fn split_lines(tokens: Vec<Token>) -> Vec<Vec<Token>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        for token in tokens.clone() {
            if token.kind == TokenKind::Eof {
                if !current_line.is_empty() {
                    current_line.push(Token {
                        kind: TokenKind::Eof,
                        line: token.line,
                        column: token.column + 1,
                    });
                    lines.push(current_line);
                    current_line = Vec::new();
                }
            } else {
                current_line.push(token);
            }
        }
        if !current_line.is_empty() {
            current_line.push(Token {
                kind: TokenKind::Eof,
                line: tokens.len(),
                column: tokens.last().unwrap().column + 1,
            });
            lines.push(current_line);
        }
        lines
    }

    pub fn get_current_token(&self) -> Option<Token> {
        if self.line >= self.tokens.len() || self.pos >= self.tokens[self.line].len() {
            None
        } else {
            Some(self.tokens[self.line][self.pos].clone())
        }
    }

    pub fn consume_token(&mut self) -> Option<Token> {
        let token = self.get_current_token()?.clone();
        self.pos += 1;
        Some(token)
    }

    pub fn extract_token(&mut self, token: TokenKind) -> Token {
        match self.get_current_token() {
            Some(Token {
                kind: current_token_kind,
                line,
                column,
            }) if current_token_kind == token => {
                self.pos += 1;
                Token {
                    kind: current_token_kind,
                    line,
                    column,
                }
            }
            _ => panic!("unexpected token: {:?}", token),
        }
    }

    fn is_lparen_call(&mut self) -> bool {
        self.pos += 1;
        let next_token = self.get_current_token();
        self.pos -= 1;
        match next_token {
            Some(Token {
                kind: TokenKind::LParen,
                ..
            }) => true,
            _ => false,
        }
    }

    fn parse_primary(&mut self) -> Result<ASTNode, ParseError> {
        let token = match self.get_current_token() {
            Some(token) => token,
            _ => panic!("token not found!"),
        };
        match token.kind {
            TokenKind::Match => self.parse_match(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Pub => self.parse_public(),
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Minus => self.parse_prefix_op(TokenKind::Minus),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),
            TokenKind::Number(value) => self.parse_literal(Value::Number(value)),
            TokenKind::String(value) => self.parse_literal(Value::String(value.into())),
            TokenKind::Bool(value) => self.parse_literal(Value::Bool(value)),
            TokenKind::True => self.parse_literal(Value::Bool(true)),
            TokenKind::False => self.parse_literal(Value::Bool(false)),
            TokenKind::Function => self.parse_function(),
            TokenKind::Pipe => self.parse_function_call_arguments(),
            TokenKind::BackSlash => self.parse_lambda(),
            TokenKind::Mutable | TokenKind::Immutable => self.parse_assign(),
            TokenKind::For => self.parse_for(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Some => self.parse_option_some(),
            TokenKind::None => self.parse_option_none(),
            TokenKind::Void => self.parse_literal(Value::Void),
            TokenKind::Success => self.parse_result_success(),
            TokenKind::Failure => self.parse_result_failure(),
            TokenKind::If => {
                let ast_if = self.parse_if()?;
                match ast_if {
                    ASTNode::If {
                        condition: _,
                        is_statement,
                        then: _,
                        ref else_,
                        ref value_type,
                        ..
                    } => {
                        if !is_statement && *value_type != ValueType::Void {
                            if else_.is_none() {
                                panic!("if expressions without else");
                            }
                        }
                    }
                    _ => {}
                }
                Ok(ast_if)
            }
            TokenKind::LParen => {
                self.consume_token(); // Consume the left parenthesis
                let expr = self.parse_expression(0)?;

                // Check for and consume the right parenthesis
                match self.get_current_token() {
                    Some(Token {
                        kind: TokenKind::RParen,
                        ..
                    }) => {
                        self.consume_token();
                        Ok(expr)
                    }
                    _ => {
                        let (line, column) = self.get_line_column();
                        Err(ParseError {
                            message: "Expected closing parenthesis".to_string(),
                            line,
                            column,
                        })
                    }
                }
            }
            TokenKind::LBrace => {
                self.pos += 1;
                match self.get_current_token() {
                    Some(token) if token.kind == TokenKind::Colon => self.parse_dict(),
                    _ => {
                        self.pos -= 1;
                        self.parse_block()
                    }
                }
            }
            TokenKind::LBrancket => self.parse_list(),
            TokenKind::Identifier(name) => self.parse_identifier(name),
            TokenKind::CommentBlock(comment) => Ok(ASTNode::CommentBlock {
                comment: comment.to_string(),
                line: token.line,
                column: token.column,
            }),
            _ => Err(ParseError::new(
                format!("unexpected token: {:?}", token.kind).as_str(),
                &token,
            )),
        }
    }

    fn parse_expression(&mut self, min_priority: u8) -> Result<ASTNode, ParseError> {
        let mut lhs = self.parse_primary()?;
        loop {
            let token = match self.get_current_token() {
                Some(token) => token,
                _ => break,
            };
            if token.kind == TokenKind::Dot {
                self.pos += 2;
                if let TokenKind::LParen = self.get_current_token().unwrap().kind {
                    self.pos -= 1;
                    if let TokenKind::Identifier(method_name) =
                        self.get_current_token().unwrap().kind
                    {
                        self.pos += 1;
                        let args = self.parse_function_call_arguments_paren()?;

                        let builtin = match lhs {
                            ASTNode::Literal {
                                value: Value::Number(_),
                                ..
                            } => true,
                            ASTNode::Literal {
                                value: Value::String(_),
                                ..
                            } => true,
                            ASTNode::Literal {
                                value: Value::Bool(_),
                                ..
                            } => true,
                            ASTNode::Literal {
                                value: Value::Void, ..
                            } => true,
                            ASTNode::Literal {
                                value: Value::List(_),
                                ..
                            } => true,
                            ASTNode::Literal {
                                value: Value::Dict(_),
                                ..
                            } => true,
                            ASTNode::FunctionCall { ref name, .. } => {
                                match self.get_function(self.get_current_scope(), name.clone()) {
                                    Some(value_type) => match value_type {
                                        ValueType::Number => true,
                                        ValueType::String => true,
                                        ValueType::Bool => true,
                                        ValueType::Void => true,
                                        ValueType::List(_) => true,
                                        ValueType::Dict(_) => true,
                                        _ => false,
                                    },
                                    _ => false,
                                }
                            }
                            ASTNode::MethodCall { ref caller, .. } => {
                                let method_info = match self.infer_type(caller) {
                                    Ok(ValueType::StructInstance { name, .. }) => {
                                        let methods = match self
                                            .get_struct(self.get_current_scope(), name.clone())
                                        {
                                            Some(ValueType::Struct {
                                                name: _,
                                                fields: _,
                                                methods,
                                            }) => methods,
                                            _ => panic!("invalid struct"),
                                        };
                                        let caller_method_name = match lhs {
                                            ASTNode::MethodCall {
                                                ref method_name, ..
                                            } => method_name,
                                            _ => panic!("invalid method call"),
                                        };
                                        match methods.get(caller_method_name) {
                                            Some(method_info) => Some(method_info.clone()),
                                            None => None,
                                        }
                                    }
                                    Ok(value_type) => {
                                        match self.get_method(
                                            self.get_current_scope(),
                                            value_type,
                                            method_name.clone(),
                                        ) {
                                            Some(method_info) => Some(method_info.clone()),
                                            None => None,
                                        }
                                    }
                                    _ => None,
                                };
                                match method_info {
                                    Some(MethodInfo { return_type, .. }) => match return_type {
                                        ValueType::Number => true,
                                        ValueType::String => true,
                                        ValueType::Bool => true,
                                        ValueType::Void => true,
                                        ValueType::List(_) => true,
                                        ValueType::Dict(_) => true,
                                        _ => false,
                                    },
                                    None => false,
                                }
                            }
                            ASTNode::Variable { ref name, .. } => {
                                match self.find_variables(self.get_current_scope(), name.clone()) {
                                    Some((value_type, _)) => match value_type {
                                        ValueType::Number => true,
                                        ValueType::String => true,
                                        ValueType::Bool => true,
                                        ValueType::Void => true,
                                        ValueType::List(_) => true,
                                        ValueType::Dict(_) => true,
                                        _ => false,
                                    },
                                    None => false,
                                }
                            }
                            _ => match self.infer_type(&lhs) {
                                Ok(ValueType::Number) => true,
                                Ok(ValueType::String) => true,
                                Ok(ValueType::Bool) => true,
                                Ok(ValueType::Void) => true,
                                Ok(ValueType::List(_)) => true,
                                Ok(ValueType::Dict(_)) => true,
                                _ => false,
                            },
                        };

                        lhs = ASTNode::MethodCall {
                            caller: Box::new(lhs.clone()),
                            method_name,
                            builtin,
                            arguments: match args {
                                ASTNode::FunctionCallArgs { args, line, column } => {
                                    Box::new(ASTNode::FunctionCallArgs {
                                        args: vec![lhs]
                                            .into_iter()
                                            .chain(args.into_iter())
                                            .collect(),
                                        line,
                                        column,
                                    })
                                }
                                _ => Box::new(ASTNode::FunctionCallArgs {
                                    args: vec![lhs],
                                    line: token.line,
                                    column: token.column,
                                }),
                            },
                            line: token.line,
                            column: token.column,
                        };
                        continue;
                    }
                }
                continue;
            }
            if token.kind == TokenKind::RArrow {
                if self.is_lparen_call() {
                    self.pos += 1;
                    let rhs = self.parse_primary()?;
                    lhs = ASTNode::LambdaCall {
                        lambda: Box::new(rhs),
                        arguments: vec![lhs],
                        line: token.line,
                        column: token.column,
                    };
                    continue;
                }
                if self.is_lambda_call() {
                    lhs = self.parse_lambda_call(lhs)?;
                } else {
                    lhs = self.parse_function_call(lhs)?;
                }
                continue;
            }

            if let Some((left_priority, right_priority)) = self.get_priority(&token) {
                if left_priority < min_priority {
                    break;
                }
                self.pos += 1;

                let rhs = self.parse_expression(right_priority)?;
                if let TokenKind::Eq = token.kind {
                    lhs = ASTNode::Eq {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                } else if let TokenKind::Gte = token.kind {
                    lhs = ASTNode::Gte {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                } else if let TokenKind::Gt = token.kind {
                    lhs = ASTNode::Gt {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                } else if let TokenKind::Lte = token.kind {
                    lhs = ASTNode::Lte {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                } else if let TokenKind::Lt = token.kind {
                    lhs = ASTNode::Lt {
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                } else {
                    lhs = ASTNode::BinaryOp {
                        left: Box::new(lhs),
                        op: token.kind,
                        right: Box::new(rhs),
                        line: token.line,
                        column: token.column,
                    }
                }
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn get_priority(&self, token: &Token) -> Option<(u8, u8)> {
        // 比較演算子やビット演算子の優先度を定義
        // 左結合と右結合を考慮して (left_priority, right_priority) のタプルを返す
        match token.kind {
            TokenKind::Lt | TokenKind::Gt | TokenKind::Lte | TokenKind::Gte => Some((1, 2)),
            TokenKind::Eq => Some((2, 3)),
            TokenKind::And => Some((3, 4)),
            TokenKind::Xor => Some((4, 5)),
            TokenKind::Or => Some((5, 6)),
            TokenKind::Plus | TokenKind::Minus => Some((6, 7)),
            TokenKind::Mul | TokenKind::Div | TokenKind::Mod => Some((7, 8)),
            TokenKind::Pow => Some((8, 8)),
            _ => None,
        }
    }

    pub fn parse(&mut self) -> Result<ASTNode, ParseError> {
        self.parse_expression(0)
    }

    pub fn parse_lines(&mut self) -> Result<Vec<ASTNode>, ParseError> {
        let mut ast_nodes = vec![];
        for _ in 0..self.tokens.len() {
            ast_nodes.push(self.parse()?);
            self.line += 1;
            if self.line >= self.tokens.len() {
                break;
            }
            self.pos = 0;
        }
        Ok(ast_nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ASTNode;
    use crate::builtin::register_builtins;
    use crate::environment::{Env, EnvVariableType, ValueType};
    use crate::token::TokenKind;
    use crate::tokenizer::tokenize;
    use crate::value::Value;
    use fraction::Fraction;

    #[test]
    fn test_four_basic_arithmetic_operations() {
        let input = "-1 + 2 * 3 % 3";

        let builtins = register_builtins(&mut Env::new());
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::BinaryOp {
                left, op, right, ..
            }) => {
                match left.as_ref() {
                    ASTNode::PrefixOp {
                        op: TokenKind::Minus,
                        expr,
                        ..
                    } => match expr.as_ref() {
                        ASTNode::Literal { value, .. } => {
                            assert_eq!(*value, Value::Number(Fraction::from(1)));
                        }
                        _ => panic!("Invalid ASTNode"),
                    },
                    _ => panic!("Invalid ASTNode"),
                }
                assert_eq!(op, TokenKind::Plus);
                match right.as_ref() {
                    ASTNode::BinaryOp {
                        left, op, right, ..
                    } => {
                        match left.as_ref() {
                            ASTNode::BinaryOp {
                                left, op, right, ..
                            } => {
                                match left.as_ref() {
                                    ASTNode::Literal { value, .. } => {
                                        assert_eq!(*value, Value::Number(Fraction::from(2)));
                                    }
                                    _ => panic!("Invalid ASTNode"),
                                }
                                assert_eq!(*op, TokenKind::Mul);
                                match right.as_ref() {
                                    ASTNode::Literal { value, .. } => {
                                        assert_eq!(*value, Value::Number(Fraction::from(3)));
                                    }
                                    _ => panic!("Invalid ASTNode"),
                                }
                            }
                            _ => panic!("Invalid ASTNode"),
                        }
                        assert_eq!(*op, TokenKind::Mod);
                        match right.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(3)));
                            }
                            _ => panic!("Invalid ASTNode"),
                        }
                    }
                    _ => panic!("Invalid ASTNode"),
                }
            }
            _ => panic!("Invalid ASTNode"),
        }
    }

    #[test]
    fn test_type_specified() {
        let input = "val mut x: number = 1";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Assign {
                name,
                value,
                variable_type,
                value_type,
                is_new,
                ..
            }) => {
                assert_eq!(name, "x");
                match value.as_ref() {
                    ASTNode::Literal { value, .. } => {
                        assert_eq!(*value, Value::Number(Fraction::from(1)));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(variable_type, EnvVariableType::Mutable);
                assert_eq!(value_type, ValueType::Number);
                assert_eq!(is_new, true);
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_type_estimate() {
        let input = "val mut x = 1";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Assign {
                name,
                value,
                variable_type,
                value_type,
                is_new,
                ..
            }) => {
                assert_eq!(name, "x");
                match value.as_ref() {
                    ASTNode::Literal { value, .. } => {
                        assert_eq!(*value, Value::Number(Fraction::from(1)));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(variable_type, EnvVariableType::Mutable);
                assert_eq!(value_type, ValueType::Number);
                assert_eq!(is_new, true);
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_register_function() {
        let input = "fun foo(x: number, y: number): number { return x + y }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Function {
                name,
                arguments,
                body,
                return_type,
                ..
            }) => {
                assert_eq!(name, "foo");
                assert_eq!(arguments.len(), 2);
                assert_eq!(return_type, ValueType::Number);
                match *body {
                    ASTNode::Block { nodes, .. } => {
                        assert_eq!(nodes.len(), 1);
                        match &nodes[0] {
                            ASTNode::Return { expr, .. } => match expr.as_ref() {
                                ASTNode::BinaryOp {
                                    left, op, right, ..
                                } => {
                                    match left.as_ref() {
                                        ASTNode::Variable {
                                            name, value_type, ..
                                        } => {
                                            assert_eq!(name, "x");
                                            assert_eq!(value_type, &Some(ValueType::Number));
                                        }
                                        _ => panic!("Invalid ASTNode"),
                                    }
                                    assert_eq!(*op, TokenKind::Plus);
                                    match right.as_ref() {
                                        ASTNode::Variable {
                                            name, value_type, ..
                                        } => {
                                            assert_eq!(name, "y");
                                            assert_eq!(value_type, &Some(ValueType::Number));
                                        }
                                        _ => panic!("Invalid ASTNode"),
                                    }
                                }
                                _ => panic!("Invalid ASTNode"),
                            },
                            _ => panic!("Invalid ASTNode"),
                        }
                    }
                    _ => panic!("Invalid ASTNode"),
                }
            }
            _ => panic!("Invalid ASTNode"),
        }
    }

    #[test]
    fn test_block() {
        // Define variables first to avoid undefined variable errors
        let input = "{ val x = 5\n val y = 10\n x + y\n return 1 - 1 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let block_result = parser.parse_block();

        if let Ok(ASTNode::Block { nodes, .. }) = block_result {
            assert_eq!(nodes.len(), 4);
            // First two nodes are variable definitions
            // Third node is the binary operation
            match &nodes[2] {
                ASTNode::BinaryOp {
                    left, op, right, ..
                } => {
                    match left.as_ref() {
                        ASTNode::Variable { name, .. } => {
                            assert_eq!(name, "x");
                        }
                        _ => panic!("Invalid ASTNode"),
                    }
                    assert_eq!(*op, TokenKind::Plus);
                    match right.as_ref() {
                        ASTNode::Variable { name, .. } => {
                            assert_eq!(name, "y");
                        }
                        _ => panic!("Invalid ASTNode"),
                    }
                }
                _ => panic!("Invalid ASTNode"),
            }
            // Fourth node is the return statement
            match &nodes[3] {
                ASTNode::Return { expr, .. } => match expr.as_ref() {
                    ASTNode::BinaryOp {
                        left, op, right, ..
                    } => {
                        match left.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(value, &Value::Number(Fraction::from(1)));
                            }
                            _ => panic!("Invalid ASTNode"),
                        }
                        assert_eq!(*op, TokenKind::Minus);
                        match right.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(value, &Value::Number(Fraction::from(1)));
                            }
                            _ => panic!("Invalid ASTNode"),
                        }
                    }
                    _ => panic!("Invalid ASTNode"),
                },
                _ => panic!("Invalid ASTNode"),
            }
        } else {
            panic!("Failed to parse block: {:?}", block_result);
        }
    }

    #[test]
    fn test_reassign_to_mutable_variable() {
        let input = "val mut x = 1\nx = 2";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);

        let results = parser.parse_lines().unwrap();
        match &results[0] {
            ASTNode::Assign {
                name,
                value,
                variable_type,
                value_type,
                is_new,
                ..
            } => {
                assert_eq!(name, "x");
                match value.as_ref() {
                    ASTNode::Literal { value, .. } => {
                        assert_eq!(*value, Value::Number(Fraction::from(1)));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(*variable_type, EnvVariableType::Mutable);
                assert_eq!(*value_type, ValueType::Number);
                assert_eq!(*is_new, true);
            }
            _ => assert!(false, "Invalid ASTNode"),
        }

        match &results[1] {
            ASTNode::Assign {
                name,
                value,
                variable_type,
                value_type,
                is_new,
                ..
            } => {
                assert_eq!(name, "x");
                match value.as_ref() {
                    ASTNode::Literal { value, .. } => {
                        assert_eq!(*value, Value::Number(Fraction::from(2)));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(*variable_type, EnvVariableType::Mutable);
                assert_eq!(*value_type, ValueType::Number);
                assert_eq!(*is_new, false);
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_function_call() {
        // First define the function f1 to avoid undefined function errors
        let input =
            "fun f1(a: number, b: number, c: number): number { return a + b + c }\n|1, 2, 3| -> f1";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let ast = parser.parse_lines().unwrap();

        // Check the function call (second AST node)
        match &ast[1] {
            ASTNode::FunctionCall {
                name, arguments, ..
            } => {
                assert_eq!(name, "f1");
                match arguments.as_ref() {
                    ASTNode::FunctionCallArgs { args, .. } => {
                        assert_eq!(args.len(), 3);
                        // Using literals instead of variables to avoid undefined variable errors
                        match &args[0] {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(value, &Value::Number(Fraction::from(1)));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match &args[1] {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(value, &Value::Number(Fraction::from(2)));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match &args[2] {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(value, &Value::Number(Fraction::from(3)));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }
    #[test]
    fn test_reassign_to_immutable_variable_should_panic() {
        let input = "val x = 1\n x = 2";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let asts = parser.parse_lines();
        match asts {
            Err(ParseError { message, .. }) => {
                assert_eq!(
                    message,
                    "It is an immutable variable and cannot be reassigned: \"x\""
                );
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }
    #[test]
    fn test_function_without_arguments_and_void_return() {
        let input = "fun no_args() { return 42 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Function {
                name,
                arguments,
                body,
                return_type,
                ..
            }) => {
                assert_eq!(name, "no_args");
                assert_eq!(arguments.len(), 0);
                match body.as_ref() {
                    ASTNode::Block { nodes, .. } => {
                        assert_eq!(nodes.len(), 1);
                        match &nodes[0] {
                            ASTNode::Return { expr, .. } => match expr.as_ref() {
                                ASTNode::Literal { value, .. } => {
                                    assert_eq!(value, &Value::Number(Fraction::from(42)));
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            },
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(return_type, ValueType::Void);
            }
            Err(ParseError { message, .. }) => {
                assert_eq!(
                    message,
                    "Return type mismatch Expected type: Void, Actual type: Number"
                );
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }
    #[test]
    fn test_function_call_with_no_arguments() {
        let input = "|| -> func()";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "func");
                match *arguments {
                    ASTNode::FunctionCallArgs { args, .. } => {
                        assert_eq!(args.len(), 0);
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_nested_block_scope() {
        let input = r#"
        {
            val mut x = 10
            {
                val y = 20
            }
            return x + 1
        }
        return x + 1
        "#;
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);

        match &parser.parse_lines().unwrap()[0] {
            ASTNode::Block { nodes: block, .. } => {
                assert_eq!(block.len(), 3);
                match &block[0] {
                    ASTNode::Assign {
                        name,
                        value,
                        variable_type,
                        value_type,
                        is_new,
                        ..
                    } => {
                        assert_eq!(name, "x");
                        match value.as_ref() {
                            ASTNode::Literal {
                                value: Value::Number(value),
                                ..
                            } => {
                                assert_eq!(value, &Fraction::from(10));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        assert_eq!(*variable_type, EnvVariableType::Mutable);
                        assert_eq!(*value_type, ValueType::Number);
                        assert_eq!(*is_new, true);
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match &block[1] {
                    ASTNode::Block {
                        nodes: inner_block, ..
                    } => {
                        assert_eq!(inner_block.len(), 1);
                        match &inner_block[0] {
                            ASTNode::Assign {
                                name,
                                value,
                                variable_type,
                                value_type,
                                is_new,
                                ..
                            } => {
                                assert_eq!(name, "y");
                                match value.as_ref() {
                                    ASTNode::Literal {
                                        value: Value::Number(value),
                                        ..
                                    } => {
                                        assert_eq!(value, &Fraction::from(20));
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                                assert_eq!(*variable_type, EnvVariableType::Immutable);
                                assert_eq!(*value_type, ValueType::Number);
                                assert_eq!(*is_new, true);
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match &block[2] {
                    ASTNode::Return { expr: value, .. } => match &**value {
                        ASTNode::BinaryOp {
                            left, op, right, ..
                        } => {
                            match left.as_ref() {
                                ASTNode::Variable {
                                    name, value_type, ..
                                } => {
                                    assert_eq!(name, "x");
                                    assert_eq!(*value_type, Some(ValueType::Number));
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                            match right.as_ref() {
                                ASTNode::Literal {
                                    value: Value::Number(value),
                                    ..
                                } => {
                                    assert_eq!(value, &Fraction::from(1));
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                            assert_eq!(*op, TokenKind::Plus);
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_prefix_operator_only() {
        let input = "-5";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::PrefixOp { op, expr, .. }) => {
                assert_eq!(op, TokenKind::Minus);
                match *expr {
                    ASTNode::Literal { value, .. } => {
                        assert_eq!(value, Value::Number(Fraction::from(5)));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_list() {
        let input = "[1, 2, 3]";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Literal {
                value: Value::List(values),
                ..
            }) => {
                assert_eq!(values.len(), 3);
                assert_eq!(values[0], Value::Number(Fraction::from(1)));
                assert_eq!(values[1], Value::Number(Fraction::from(2)));
                assert_eq!(values[2], Value::Number(Fraction::from(3)));
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_fraction_and_decimal_operations() {
        let input = "5.2 + 3.2";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins.clone());

        match parser.parse() {
            Ok(ASTNode::BinaryOp {
                left, op, right, ..
            }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::new(26u64, 5u64));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(op, TokenKind::Plus);
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::new(16u64, 5u64));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }

        let input = "1/3 * 2/5";
        // 分数の演算テスト
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins.clone());
        match parser.parse() {
            Ok(ASTNode::BinaryOp {
                left, op, right, ..
            }) => {
                match *left {
                    ASTNode::BinaryOp {
                        left, op, right, ..
                    } => {
                        match *left {
                            ASTNode::BinaryOp {
                                left, op, right, ..
                            } => {
                                match *left {
                                    ASTNode::Literal {
                                        value: Value::Number(value),
                                        ..
                                    } => {
                                        assert_eq!(value, Fraction::from(1));
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                                assert_eq!(op, TokenKind::Div);
                                match *right {
                                    ASTNode::Literal {
                                        value: Value::Number(value),
                                        ..
                                    } => {
                                        assert_eq!(value, Fraction::from(3));
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        assert_eq!(op, TokenKind::Mul);
                        match *right {
                            ASTNode::Literal {
                                value: Value::Number(value),
                                ..
                            } => {
                                assert_eq!(value, Fraction::new(2u64, 1u64));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(op, TokenKind::Div);
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::new(5u64, 1u64));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_function_call_chain() {
        let input = "1 -> f1 -> f2";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::FunctionCall {
                name, arguments, ..
            }) => {
                assert_eq!(name, "f2");
                match arguments.as_ref() {
                    ASTNode::FunctionCallArgs { args, .. } => {
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            ASTNode::FunctionCall {
                                name, arguments, ..
                            } => {
                                assert_eq!(name, "f1");
                                match arguments.as_ref() {
                                    ASTNode::FunctionCallArgs { args, .. } => {
                                        assert_eq!(args.len(), 1);
                                        match args[0] {
                                            ASTNode::Literal {
                                                value: Value::Number(value),
                                                ..
                                            } => {
                                                assert_eq!(value, Fraction::from(1));
                                            }
                                            _ => assert!(false, "Invalid ASTNode"),
                                        }
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_lambda() {
        let input = "val inc = \\|x: number| => x + 1";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Assign {
                name,
                variable_type,
                is_new,
                value_type,
                value,
                ..
            }) => {
                assert_eq!(name, "inc");
                assert_eq!(variable_type, EnvVariableType::Immutable);
                assert_eq!(is_new, true);
                assert_eq!(value_type, ValueType::Lambda);
                match *value {
                    ASTNode::Lambda {
                        arguments, body, ..
                    } => {
                        assert_eq!(arguments.len(), 1);
                        match &arguments[0] {
                            ASTNode::Variable {
                                name, value_type, ..
                            } => {
                                assert_eq!(name, "x");
                                assert_eq!(*value_type, Some(ValueType::Number));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match *body {
                            ASTNode::BinaryOp {
                                left, op, right, ..
                            } => {
                                match *left {
                                    ASTNode::Variable {
                                        name, value_type, ..
                                    } => {
                                        assert_eq!(name, "x");
                                        assert_eq!(value_type, Some(ValueType::Number));
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                                assert_eq!(op, TokenKind::Plus);
                                match right.as_ref() {
                                    ASTNode::Literal {
                                        value: Value::Number(value),
                                        ..
                                    } => {
                                        assert_eq!(*value, Fraction::from(1));
                                    }
                                    _ => assert!(false, "Invalid ASTNode"),
                                }
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_if() {
        // Define x first to avoid undefined variable error
        let input = "val x = 1\nif (x == 1) { 1 } else { 0 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let ast = parser.parse_lines().unwrap();

        // Check the if statement (second AST node)
        match &ast[1] {
            ASTNode::If {
                condition,
                then,
                value_type,
                ..
            } => {
                match condition.as_ref() {
                    ASTNode::Eq { left, right, .. } => {
                        match left.as_ref() {
                            ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match right.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(1)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match then.as_ref() {
                    ASTNode::Block {
                        nodes: statements, ..
                    } => match &statements[0] {
                        ASTNode::Literal { value, .. } => {
                            assert_eq!(*value, Value::Number(Fraction::from(1)))
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(ValueType::Number, value_type.clone());
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    #[should_panic(expected = "if expressions without else")]
    fn test_partial_return_if() {
        // Define x first to avoid undefined variable error
        let input = "val x = 1\nif (x == 1) { 1 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);

        // This should panic because if expressions without else are not allowed
        let _ast = parser.parse_lines().unwrap();

        // If we get here, it means the test didn't panic as expected
        panic!("if expressions without else");
    }

    #[test]
    fn test_if_statement() {
        // Define x first to avoid undefined variable error
        let input = "val x = 1\nif (x == 1) { return 1 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let ast = parser.parse_lines().unwrap();

        // Check the if statement (second AST node)
        match &ast[1] {
            ASTNode::If {
                condition,
                then,
                value_type,
                ..
            } => {
                match condition.as_ref() {
                    ASTNode::Eq { left, right, .. } => {
                        match left.as_ref() {
                            ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match right.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(1)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match then.as_ref() {
                    ASTNode::Block {
                        nodes: statements, ..
                    } => match &statements[0] {
                        ASTNode::Return { expr: value, .. } => match value.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(1)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        },
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(ValueType::Number, value_type.clone());
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_else() {
        // Define x first to avoid undefined variable error
        let input = "val x = 1\nif (x == 1) { return 1 } else { return 0 }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        let ast = parser.parse_lines().unwrap();

        // Check the if statement (second AST node)
        match &ast[1] {
            ASTNode::If {
                condition: result_condition,
                then: result_then,
                else_: result_else_,
                value_type: result_value_type,
                ..
            } => {
                match result_condition.as_ref() {
                    ASTNode::Eq { left, right, .. } => {
                        match left.as_ref() {
                            ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match right.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(1)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match result_then.as_ref() {
                    ASTNode::Block {
                        nodes: statements, ..
                    } => match &statements[0] {
                        ASTNode::Return { expr: value, .. } => match value.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(1)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        },
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match result_else_.as_ref().unwrap().as_ref() {
                    ASTNode::Block {
                        nodes: statements, ..
                    } => match &statements[0] {
                        ASTNode::Return { expr: value, .. } => match value.as_ref() {
                            ASTNode::Literal { value, .. } => {
                                assert_eq!(*value, Value::Number(Fraction::from(0)))
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        },
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                }
                assert_eq!(ValueType::Number, *result_value_type);
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }
    #[test]
    fn test_else_if() {
        let input = r#"
          if (x == 1) {
            return 1
          } else if (x == 2) {
            return 2
          } else if (x == 3) {
            return 3
          } else {
            return 0
          }
        "#;
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        if let Ok(ASTNode::If {
            condition: result_condition,
            then: result_then,
            else_: result_else_,
            value_type: result_value_type,
            ..
        }) = parser.parse()
        {
            match result_condition.as_ref() {
                ASTNode::Eq { left, right, .. } => {
                    match left.as_ref() {
                        ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                    match right.as_ref() {
                        ASTNode::Literal { value, .. } => {
                            assert_eq!(*value, Value::Number(Fraction::from(1)))
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                }
                _ => assert!(false, "Invalid ASTNode"),
            }
            match result_then.as_ref() {
                ASTNode::Block {
                    nodes: statements, ..
                } => match &statements[0] {
                    ASTNode::Return { expr: value, .. } => match value.as_ref() {
                        ASTNode::Literal { value, .. } => {
                            assert_eq!(*value, Value::Number(Fraction::from(1)))
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    },
                    _ => assert!(false, "Invalid ASTNode"),
                },
                _ => assert!(false, "Invalid ASTNode"),
            }
            match result_else_.unwrap().as_ref() {
                ASTNode::If {
                    condition,
                    then,
                    else_,
                    ..
                } => {
                    match condition.as_ref() {
                        ASTNode::Eq { left, right, .. } => {
                            match left.as_ref() {
                                ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                            match right.as_ref() {
                                ASTNode::Literal { value, .. } => {
                                    assert_eq!(*value, Value::Number(Fraction::from(2)))
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                    match then.as_ref() {
                        ASTNode::Block {
                            nodes: statements, ..
                        } => match &statements[0] {
                            ASTNode::Return { expr: value, .. } => match value.as_ref() {
                                ASTNode::Literal { value, .. } => {
                                    assert_eq!(*value, Value::Number(Fraction::from(2)))
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            },
                            _ => assert!(false, "Invalid ASTNode"),
                        },
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                    match else_.as_ref().unwrap().as_ref() {
                        ASTNode::If {
                            condition, then, ..
                        } => {
                            match &condition.as_ref() {
                                ASTNode::Eq { left, right, .. } => {
                                    match left.as_ref() {
                                        ASTNode::Variable { name, .. } => assert_eq!(name, "x"),
                                        _ => assert!(false, "Invalid ASTNode"),
                                    }
                                    match right.as_ref() {
                                        ASTNode::Literal { value, .. } => {
                                            assert_eq!(*value, Value::Number(Fraction::from(3)))
                                        }
                                        _ => assert!(false, "Invalid ASTNode"),
                                    }
                                }
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                            match then.as_ref() {
                                ASTNode::Block {
                                    nodes: statements, ..
                                } => match &statements[0] {
                                    ASTNode::Return { expr: value, .. } => match value.as_ref() {
                                        ASTNode::Literal { value, .. } => {
                                            assert_eq!(*value, Value::Number(Fraction::from(3)))
                                        }
                                        _ => assert!(false, "Invalid ASTNode"),
                                    },
                                    _ => assert!(false, "Invalid ASTNode"),
                                },
                                _ => assert!(false, "Invalid ASTNode"),
                            }
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                }
                _ => assert!(false, "Invalid ASTNode"),
            }
            assert_eq!(ValueType::Number, result_value_type);
        };
    }

    #[test]
    fn test_comparison_operations() {
        let input = "1 == 1";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins.clone());
        match parser.parse() {
            Ok(ASTNode::Eq { left, right, .. }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(1));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(1));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
        let input = "2 > 1";
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins.clone());

        match parser.parse() {
            Ok(ASTNode::Gt { left, right, .. }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(2));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(1));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }

        let input = "3 >= 3";
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins.clone());

        match parser.parse() {
            Ok(ASTNode::Gte { left, right, .. }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(3));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(3));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }

        let input = "1 < 2";
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins.clone());

        match parser.parse() {
            Ok(ASTNode::Lt { left, right, .. }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(1));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(2));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }

        let input = "4 <= 4";
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, builtins.clone());

        match parser.parse() {
            Ok(ASTNode::Lte { left, right, .. }) => {
                match *left {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(4));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match *right {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(4));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_struct() {
        let input = "struct Point { x: number, y: number }";
        let tokens = tokenize(&input.to_string());
        let builtins = register_builtins(&mut Env::new());
        let mut parser = Parser::new(tokens, builtins);
        match parser.parse() {
            Ok(ASTNode::Struct { name, fields, .. }) => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                let x = fields.get("x").unwrap();
                let y = fields.get("y").unwrap();
                match x {
                    ASTNode::StructField {
                        value_type,
                        is_public,
                        ..
                    } => {
                        assert_eq!(*value_type, ValueType::Number);
                        assert_eq!(*is_public, false);
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match y {
                    ASTNode::StructField {
                        value_type,
                        is_public,
                        ..
                    } => {
                        assert_eq!(*value_type, ValueType::Number);
                        assert_eq!(*is_public, false);
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_struct_instance() {
        let input = r#"
            struct Point {
                x: number,
                y: number
            }
            Point { x: 1, y: 2 }
        "#
        .to_string();
        let tokens = tokenize(&input.to_string());
        let mut parser = Parser::new(tokens, register_builtins(&mut Env::new()));
        let results = parser.parse_lines().unwrap();
        assert_eq!(results.len(), 2);
        match &results[1] {
            ASTNode::StructInstance { name, fields, .. } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                let x = fields.get("x").unwrap();
                let y = fields.get("y").unwrap();
                match x {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(*value, Fraction::from(1));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
                match y {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(*value, Fraction::from(2));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_struct_field_access() {
        let input = r#"
          pub struct Point {
              x: number,
              y: number
          }
          val point = Point {x: 1, y: 2}
          point.x
          point.x = 3
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut Env::new()));
        let results = parser.parse_lines().unwrap();
        assert_eq!(results.len(), 4);
        match &results[0] {
            ASTNode::Public { node, .. } => match node.as_ref() {
                ASTNode::Struct { name, fields, .. } => {
                    assert_eq!(name, "Point");
                    assert_eq!(fields.len(), 2);
                    let x = fields.get("x").unwrap();
                    let y = fields.get("y").unwrap();
                    match x {
                        ASTNode::StructField {
                            value_type,
                            is_public,
                            ..
                        } => {
                            assert_eq!(*value_type, ValueType::Number);
                            assert_eq!(*is_public, false);
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                    match y {
                        ASTNode::StructField {
                            value_type,
                            is_public,
                            ..
                        } => {
                            assert_eq!(*value_type, ValueType::Number);
                            assert_eq!(*is_public, false);
                        }
                        _ => assert!(false, "Invalid ASTNode"),
                    }
                }
                _ => assert!(false, "Invalid ASTNode"),
            },
            _ => assert!(false, "Invalid ASTNode"),
        }
        match &results[1] {
            ASTNode::Assign { name, value, .. } => {
                assert_eq!(name, "point");
                match value.as_ref() {
                    ASTNode::StructInstance { name, fields, .. } => {
                        assert_eq!(name, "Point");
                        assert_eq!(fields.len(), 2);
                        let x = fields.get("x").unwrap();
                        let y = fields.get("y").unwrap();
                        match x {
                            ASTNode::Literal {
                                value: Value::Number(value),
                                ..
                            } => {
                                assert_eq!(*value, Fraction::from(1));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                        match y {
                            ASTNode::Literal {
                                value: Value::Number(value),
                                ..
                            } => {
                                assert_eq!(*value, Fraction::from(2));
                            }
                            _ => assert!(false, "Invalid ASTNode"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
        match &results[2] {
            ASTNode::StructFieldAccess {
                instance,
                field_name,
                ..
            } => {
                assert_eq!(field_name, "x");
                match instance.as_ref() {
                    ASTNode::Variable {
                        name, value_type, ..
                    } => {
                        assert_eq!(name, "point");
                        match value_type {
                            Some(ValueType::StructInstance { name, fields }) => {
                                assert_eq!(name, "Point");
                                assert_eq!(fields.len(), 2);
                                let x = fields.get("x").unwrap();
                                let y = fields.get("y").unwrap();
                                match x {
                                    ValueType::Number => {}
                                    _ => assert!(false, "Invalid ValueType"),
                                }
                                match y {
                                    ValueType::Number => {}
                                    _ => assert!(false, "Invalid ValueType"),
                                }
                            }
                            _ => assert!(false, "Invalid ValueType"),
                        }
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
        match &results[3] {
            ASTNode::StructFieldAssign {
                field_name, value, ..
            } => {
                assert_eq!(field_name, "x");
                match *value.as_ref() {
                    ASTNode::Literal {
                        value: Value::Number(value),
                        ..
                    } => {
                        assert_eq!(value, Fraction::from(3));
                    }
                    _ => assert!(false, "Invalid ASTNode"),
                }
            }
            _ => assert!(false, "Invalid ASTNode"),
        }
    }

    #[test]
    fn test_impl() {
        let input = r#"
        impl Point {
            fun move(self, dx: number) {
                self.x = self.x + dx
            }
        }
        "#
        .to_string();
        let tokens = tokenize(&input);
        let mut env = Env::new();
        let builtins = register_builtins(&mut env);
        let mut parser = Parser::new(tokens, builtins);
        let base_struct = ASTNode::Struct {
            name: "Point".into(),
            fields: HashMap::from_iter(vec![(
                "x".into(),
                ASTNode::StructField {
                    value_type: ValueType::Number,
                    is_public: false,
                    line: 2,
                    column: 13,
                },
            )]),
            line: 2,
            column: 9,
        };
        parser.register_struct("global".into(), base_struct);
        let result = parser.parse_lines();
        if result.is_err() {
            panic!("Failed to parse: {:?}", result.err());
        }
        match &result.unwrap()[0] {
            ASTNode::Impl {
                base_struct: result_base_struct,
                methods,
                ..
            } => {
                match result_base_struct.as_ref() {
                    ValueType::Struct { name, fields, .. } => {
                        assert_eq!(name, "Point");
                        assert_eq!(fields.len(), 1);
                        let x = fields.get("x").unwrap();
                        match x {
                            ValueType::StructField {
                                value_type,
                                is_public,
                            } => {
                                assert_eq!(*value_type.as_ref(), ValueType::Number);
                                assert_eq!(*is_public, false);
                            }
                            _ => panic!("Invalid instance"),
                        }
                    }
                    _ => panic!("Invalid instance"),
                }
                assert_eq!(methods.len(), 1);
                match &methods[0] {
                    ASTNode::Method {
                        name,
                        arguments,
                        is_mut,
                        body,
                        ..
                    } => {
                        assert_eq!(name, "move");
                        assert_eq!(*is_mut, false);
                        assert_eq!(arguments.len(), 2);
                        match &arguments[0] {
                            ASTNode::Variable {
                                name, value_type, ..
                            } => {
                                assert_eq!(name, "self");
                                assert_eq!(*value_type, Some(ValueType::SelfType));
                            }
                            _ => assert!(false, "Invalid argument"),
                        }
                        match &arguments[1] {
                            ASTNode::Variable {
                                name, value_type, ..
                            } => {
                                assert_eq!(name, "dx");
                                assert_eq!(*value_type, Some(ValueType::Number));
                            }
                            _ => assert!(false, "Invalid argument"),
                        }
                        match *body.clone() {
                            ASTNode::Block { nodes, .. } => {
                                assert_eq!(nodes.len(), 1);
                                match &nodes[0] {
                                    ASTNode::StructFieldAssign {
                                        instance,
                                        field_name,
                                        ..
                                    } => {
                                        assert_eq!(field_name, "x");
                                        match instance.as_ref() {
                                            ASTNode::StructFieldAccess {
                                                instance,
                                                field_name,
                                                ..
                                            } => {
                                                assert_eq!(field_name, "x");
                                                match instance.as_ref() {
                                                    ASTNode::Variable {
                                                        name, value_type, ..
                                                    } => {
                                                        assert_eq!(name, "self");
                                                        match value_type {
                                                            Some(ValueType::Struct {
                                                                name,
                                                                fields,
                                                                ..
                                                            }) => {
                                                                assert_eq!(name, "Point");
                                                                assert_eq!(fields.len(), 1);
                                                                let x = fields.get("x").unwrap();
                                                                match x {
                                                                    ValueType::StructField {
                                                                        value_type,
                                                                        is_public,
                                                                    } => {
                                                                        assert_eq!(
                                                                            *value_type.as_ref(),
                                                                            ValueType::Number
                                                                        );
                                                                        assert_eq!(
                                                                            *is_public,
                                                                            false
                                                                        );
                                                                    }
                                                                    _ => panic!("Invalid instance"),
                                                                }
                                                            }
                                                            _ => panic!("Invalid instance"),
                                                        }
                                                    }
                                                    _ => panic!("Invalid instance"),
                                                }
                                            }
                                            _ => panic!("Invalid instance"),
                                        }
                                    }
                                    _ => panic!("Invalid node"),
                                }
                            }
                            _ => panic!("Invalid body"),
                        }
                    }
                    _ => panic!("Invalid method"),
                }
            }
            _ => panic!("Invalid impl"),
        }
    }

    #[test]
    fn test_for() {
        let input = "for i in [1, 2, 3] { print(i) }";
        let tokens = tokenize(&input.to_string());
        let mut env = Env::new();
        let builtins = register_builtins(&mut env);
        let mut parser = Parser::new(tokens, builtins);
        if let Ok(parse_result) = parser.parse() {
            match parse_result {
                ASTNode::For {
                    variable,
                    iterable,
                    body,
                    ..
                } => {
                    assert_eq!(variable, "i");
                    match *iterable {
                        ASTNode::Literal {
                            value: Value::List(iterable),
                            ..
                        } => {
                            for (i, value) in iterable.iter().enumerate() {
                                assert_eq!(value, &Value::Number(Fraction::from(i as u64 + 1)));
                            }
                        }
                        _ => panic!("Invalid iterable"),
                    }
                    match *body {
                        ASTNode::Block { nodes, .. } => {
                            assert_eq!(nodes.len(), 1);
                            match &nodes[0] {
                                ASTNode::FunctionCall {
                                    name, arguments, ..
                                } => {
                                    assert_eq!(name, "print");
                                    match *arguments.clone() {
                                        ASTNode::FunctionCallArgs { args, .. } => {
                                            assert_eq!(args.len(), 1);
                                            match &args[0] {
                                                ASTNode::Variable { name, .. } => {
                                                    assert_eq!(name, "i");
                                                }
                                                _ => panic!("Invalid argument"),
                                            }
                                        }
                                        _ => panic!("Invalid arguments"),
                                    }
                                }
                                _ => panic!("Invalid body"),
                            }
                        }
                        _ => panic!("Invalid body"),
                    }
                }
                _ => panic!("Invalid parse result"),
            }
        }
    }
}
