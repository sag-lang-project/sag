use crate::ast::ASTNode;
use crate::environment::ValueType;
use crate::parsers::parse_error::ParseError;
use crate::parsers::Parser;
use crate::token::{Token, TokenKind};
use crate::value::Value;

impl Parser {
    pub fn parse_method(&mut self) -> Result<ASTNode, ParseError> {
        self.consume_token();
        let name = match self.get_current_token() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                ..
            }) => name,
            _ => panic!("unexpected token"),
        };
        self.enter_scope(name.to_string());
        // メソッドスコープに入る
        self.enter_method_scope();

        self.consume_token();
        self.extract_token(TokenKind::LParen);
        let arguments = self.parse_function_arguments()?;
        let mut is_mut = false;
        // Check if this is a static method (no self parameter) or instance method
        let _is_static_method = if arguments.len() > 0 {
            match arguments.first() {
                Some(ASTNode::Variable {
                    name, value_type, ..
                }) => {
                    if name == "self" {
                        match value_type {
                            Some(value_type) => {
                                is_mut = *value_type != ValueType::SelfType;
                            }
                            _ => {}
                        }
                        false // Not static, it has self
                    } else {
                        true // Static method, first param is not self
                    }
                }
                _ => true, // Not a variable, so not self
            }
        } else {
            true // No arguments, so static method
        };
        let return_type = self.parse_return_type();
        let body = self.parse_block()?;
        // メソッドスコープから出る
        self.leave_method_scope();
        self.leave_scope();
        let (line, column) = self.get_line_column();
        let method = ASTNode::Method {
            name: name.clone(),
            arguments,
            body: Box::new(body),
            return_type,
            is_mut,
            line,
            column,
        };
        self.register_method(
            self.get_current_scope(),
            self.current_struct.clone().unwrap(),
            method.clone(),
        );
        Ok(method)
    }

    fn is_builtin_method(&self, caller: &ASTNode) -> bool {
        let builtin = match caller {
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
            ASTNode::Variable {
                name, value_type, ..
            } => {
                if value_type.is_none() {
                    let variable = self.find_variables(self.get_current_scope(), name.clone());
                    match variable {
                        Some((value_type, _)) => match value_type {
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
                } else {
                    match value_type {
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
            }
            ASTNode::MethodCall { caller, .. } => match self.infer_type(&caller) {
                Ok(ValueType::Number) => true,
                Ok(ValueType::String) => true,
                Ok(ValueType::Bool) => true,
                Ok(ValueType::Void) => true,
                Ok(ValueType::List(_)) => true,
                Ok(ValueType::Dict(_)) => true,
                _ => false,
            },
            _ => false,
        };
        builtin
    }

    pub fn parse_method_call(
        &mut self,
        caller: ASTNode,
        method_name: String,
        arguments: ASTNode,
    ) -> Result<ASTNode, ParseError> {
        let builtin = self.is_builtin_method(&caller);
        let (line, column) = self.get_line_column();
        Ok(ASTNode::MethodCall {
            method_name,
            caller: Box::new(caller),
            arguments: Box::new(arguments),
            builtin,
            line,
            column,
        })
    }
}
