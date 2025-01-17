pub mod prefix_op;
pub mod struct_node;
pub mod function_node;
pub mod comparison_op;
pub mod if_node;
pub mod assign_node;
pub mod lambda_node;
pub mod variable_node;
pub mod binary_op;
pub mod for_node;
pub mod import_node;
pub mod method_call_node;
pub mod runtime_error;

use crate::environment::Env;
use crate::ast::ASTNode;
use crate::value::Value;
use crate::token::TokenKind;
use crate::evals::runtime_error::RuntimeError;

pub fn evals(asts: Vec<ASTNode>, env: &mut Env) -> Result<Vec<Value>, RuntimeError> {
    let mut values = vec![];
    for ast in asts {
        values.push(eval(ast, env)?);
    }
    Ok(values)
}

pub fn eval(ast: ASTNode, env: &mut Env) -> Result<Value, RuntimeError> {
    match ast {
        ASTNode::Import {
            module_name,
            symbols,
            line,
            column,
        } => import_node::import_node(module_name, symbols, line, column, env),
        ASTNode::Public { node, line, column } => import_node::public_node(node, line, column, env),
        ASTNode::Literal{value, ..} => Ok(value.clone()),
        ASTNode::PrefixOp { op, expr, line, column } => prefix_op::prefix_op(op, expr, line, column, env),
        ASTNode::Struct {
            name,
            fields,
            line,
            column,
        } => struct_node::struct_node(name, fields, line, column, env),
        ASTNode::Impl {
            base_struct,
            methods,
            line,
            column,
        } => {
            struct_node::impl_node(base_struct, methods, line, column, env)
        }
        ASTNode::MethodCall { method_name, caller, arguments, builtin, line, column } => {
            if builtin {
                method_call_node::builtin_method_call_node(method_name, caller, arguments, line, column, env)
            } else {
                match caller {
                    _ => method_call_node::method_call_node(method_name, caller, arguments, line, column, env)
                }
            }
        }
        ASTNode::StructInstance {
            name,
            fields,
            line,
            column,
        } => {
            struct_node::struct_instance_node(name, fields, line, column, env)
        }
        ASTNode::StructFieldAssign { instance, field_name: updated_field_name, value: updated_value_ast, line, column } => {
            struct_node::struct_field_assign_node(instance, updated_field_name, updated_value_ast, line, column, env)
        }
        ASTNode::StructFieldAccess { instance, field_name, line, column } => {
            struct_node::struct_field_access_node(instance, field_name, line, column, env)
        }
        ASTNode::Function {
            name,
            arguments,
            body,
            return_type,
            line,
            column,
        } => {
            function_node::function_node(name, arguments, body, return_type, line, column, env)
        }
        ASTNode::Lambda { arguments, body, .. } => Ok(Value::Lambda {
            arguments,
            body: body.clone(),
            env: env.clone(),
        }),
        ASTNode::Block{nodes: statements, line, column} => {
            function_node::block_node(statements, line, column, env)
        }
        ASTNode::Return{expr: value, line, column} => {
            Ok(Value::Return(Box::new(eval(*value, env)?)))
        }
        ASTNode::Eq { left, right, line, column } => {
            comparison_op::comparison_op_node(TokenKind::Eq, left, right, line, column, env)
        }
        ASTNode::Gte { left, right, line, column, } => {
            comparison_op::comparison_op_node(TokenKind::Gte, left, right, line, column, env)
        }
        ASTNode::Gt { left, right, line, column, } => {
            comparison_op::comparison_op_node(TokenKind::Gt, left, right, line, column, env)
        }
        ASTNode::Lte { left, right, line, column, } => {
            comparison_op::comparison_op_node(TokenKind::Lte, left, right, line, column, env)
        }
        ASTNode::Lt { left, right, line, column, } => {
            comparison_op::comparison_op_node(TokenKind::Lt, left, right, line, column, env)
        }
        ASTNode::For {
            variable,
            iterable,
            body,
            line,
            column
        } => {
            for_node::for_node(variable, iterable, body, line, column, env)
        }
        ASTNode::If {
            condition,
            then,
            else_,
            value_type: _,
            line,
            column
        } => {
            if_node::if_node(condition, then, else_, line, column, env)
        }
        ASTNode::Assign {
            name,
            value,
            variable_type,
            value_type: _,
            is_new,
            line,
            column
        } => {
            assign_node::assign_node(name, value, variable_type, is_new, line, column, env)
        }
        ASTNode::LambdaCall { lambda, arguments, line, column } => {
            lambda_node::lambda_call_node(lambda, arguments, line, column, env)
        }
        ASTNode::FunctionCall { name, arguments, line, column } => {
            function_node::function_call_node(name, arguments, line, column, env)
        }
        ASTNode::Variable {
            name,
            value_type,
            line,
            column,
        } => {
            variable_node::variable_node(name, value_type, line, column, env)
        }
        ASTNode::BinaryOp { left, op, right, line, column } => {
            binary_op::binary_op(op, left, right, line, column, env)
        }
        ASTNode::CommentBlock{..} => Ok(Value::Void),
        _ => Err(RuntimeError::new(format!("Unsupported ast node: {:?}", ast).as_str(), 0, 0)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use crate::tokenizer::tokenize;
    use crate::parsers::Parser;
    use crate::environment::EnvVariableType;
    use crate::builtin::register_builtins;
    use fraction::Fraction;
    use crate::token::Token;
    use crate::environment::ValueType;

    #[test]
    fn test_four_basic_arithmetic_operations() {
        let mut env = Env::new();
        let ast = ASTNode::BinaryOp {
            left: Box::new(ASTNode::PrefixOp {
                op: TokenKind::Minus,
                expr: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            }),
            op: TokenKind::Plus,
            right: Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(2)))),
                op: TokenKind::Mul,
                right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(3)))),
            }),
        };
        assert_eq!(Value::Number(Fraction::from(5)), eval(ast, &mut env));
    }
    #[test]
    fn test_assign() {
        let mut env = Env::new();
        let ast = ASTNode::Assign {
            name: "x".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(5)))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        assert_eq!(Value::Number(Fraction::from(5)), eval(ast, &mut env));
        assert_eq!(
            Value::Number(Fraction::from(5)),
            env.get(&"x".to_string(), None).unwrap().value
        );
        assert_eq!(
            EnvVariableType::Mutable,
            env.get(&"x".to_string(), None).unwrap().variable_type
        );
        let mut env = Env::new();
        let ast = ASTNode::Assign {
            name: "x".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(5)))),
            variable_type: EnvVariableType::Immutable,
            value_type: ValueType::Number,
            is_new: false,
        };
        assert_eq!(Value::Number(Fraction::from(5)), eval(ast, &mut env));
        assert_eq!(
            Value::Number(Fraction::from(5)),
            env.get(&"x".to_string(), None).unwrap().value
        );
        assert_eq!(
            EnvVariableType::Immutable,
            env.get(&"x".to_string(), None).unwrap().variable_type
        );
    }
    #[test]
    fn test_assign_expression_value() {
        let mut env = Env::new();
        let ast = ASTNode::Assign {
            name: "y".to_string(),
            value: Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(10)))),
                op: TokenKind::Plus,
                right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(20)))),
            }),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        assert_eq!(Value::Number(Fraction::from(30)), eval(ast, &mut env));
        assert_eq!(
            env.get(&"y".to_string(), None).unwrap().value,
            Value::Number(Fraction::from(30))
        );
    }
    #[test]
    fn test_assign_overwrite_mutable_variable() {
        let mut env = Env::new();

        let ast1 = ASTNode::Assign {
            name: "z".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(50)))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        eval(ast1, &mut env);

        // 再代入
        let ast2 = ASTNode::Assign {
            name: "z".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(100)))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: false,
        };

        // 環境に新しい値が登録されていること
        assert_eq!(eval(ast2, &mut env), Value::Number(Fraction::from(100)));
        assert_eq!(
            env.get(&"z".to_string(), None).unwrap().value,
            Value::Number(Fraction::from(100))
        );
    }
    #[test]
    #[should_panic(expected = "Cannot reassign to immutable variable")]
    fn test_assign_to_immutable_variable() {
        let mut env = Env::new();

        // Immutable 変数の初期値を設定
        let ast1 = ASTNode::Assign {
            name: "w".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(200)))),
            variable_type: EnvVariableType::Immutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        eval(ast1, &mut env);

        // 再代入しようとしてエラー
        let ast2 = ASTNode::Assign {
            name: "w".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(300)))),
            variable_type: EnvVariableType::Immutable,
            value_type: ValueType::Number,
            is_new: false,
        };
        eval(ast2, &mut env);
    }
    #[test]
    fn test_register_function_and_function_call() {
        let mut env = Env::new();
        let ast = ASTNode::Function {
            name: "foo".into(),
            arguments: vec![
                ASTNode::Variable {
                    name: "x".into(),
                    value_type: Some(ValueType::Number),
                },
                ASTNode::Variable {
                    name: "y".into(),
                    value_type: Some(ValueType::Number),
                },
            ],
            body: Box::new(ASTNode::Block(vec![ASTNode::Return(Box::new(
                ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Variable {
                        name: "x".into(),
                        value_type: Some(ValueType::Number),
                    }),
                    op: TokenKind::Plus,
                    right: Box::new(ASTNode::Variable {
                        name: "y".into(),
                        value_type: Some(ValueType::Number),
                    }),
                },
            ))])),
            return_type: ValueType::Number,
        };
        eval(ast, &mut env);
        let ast = ASTNode::FunctionCall {
            name: "foo".into(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![
                ASTNode::Literal(Value::Number(Fraction::from(1))),
                ASTNode::Literal(Value::Number(Fraction::from(2))),
            ])),
        };
        let result = eval(ast, &mut env);
        assert_eq!(result, Value::Number(Fraction::from(3)));
    }

    #[test]
    #[should_panic(expected = "Unexpected prefix op: Plus")]
    fn test_unsupported_prefix_operation() {
        let mut env = Env::new();
        let ast = ASTNode::PrefixOp {
            op: TokenKind::Plus,
            expr: Box::new(ASTNode::Literal(Value::Number(Fraction::from(5)))),
        };
        eval(ast, &mut env);
    }

    #[test]
    #[should_panic(expected = "Unsupported operation")]
    fn test_unsupported_binary_operation() {
        let mut env = Env::new();
        let ast = ASTNode::BinaryOp {
            left: Box::new(ASTNode::Literal(Value::String("hello".to_string()))),
            op: TokenKind::Mul,
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(5)))),
        };
        eval(ast, &mut env);
    }

    #[test]
    fn test_list() {
        let mut env = Env::new();
        let ast = ASTNode::Assign {
            name: "x".to_string(),
            value: Box::new(ASTNode::Literal(Value::List(vec![
                Value::Number(Fraction::from(1)),
                Value::Number(Fraction::from(2)),
                Value::Number(Fraction::from(3)),
            ]))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::List(Box::new(ValueType::Number)),
            is_new: true,
        };
        assert_eq!(
            Value::List(vec![
                Value::Number(Fraction::from(1)),
                Value::Number(Fraction::from(2)),
                Value::Number(Fraction::from(3)),
            ]),
            eval(ast, &mut env)
        );
        assert_eq!(
            Value::List(vec![
                Value::Number(Fraction::from(1)),
                Value::Number(Fraction::from(2)),
                Value::Number(Fraction::from(3)),
            ]),
            env.get(&"x".to_string(), None).unwrap().value
        );
    }

    #[test]
    #[should_panic(expected = "does not match arguments length")]
    fn test_function_call_argument_mismatch() {
        let mut env = Env::new();
        let ast_function = ASTNode::Function {
            name: "bar".to_string(),
            arguments: vec![ASTNode::Variable {
                name: "x".into(),
                value_type: Some(ValueType::Number),
            }],
            body: Box::new(ASTNode::Return(Box::new(ASTNode::Variable {
                name: "x".into(),
                value_type: Some(ValueType::Number),
            }))),
            return_type: ValueType::Number,
        };
        eval(ast_function, &mut env);

        // 引数の数が合わない関数呼び出し
        let ast_call = ASTNode::FunctionCall {
            name: "bar".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![
                ASTNode::Literal(Value::Number(Fraction::from(5))),
                ASTNode::Literal(Value::Number(Fraction::from(10))), // 余分な引数
            ])),
        };
        eval(ast_call, &mut env);
    }

    #[test]
    fn test_scope_management_in_function() {
        let mut env = Env::new();

        // 関数定義
        let ast_function = ASTNode::Function {
            name: "add_and_return".to_string(),
            arguments: vec![ASTNode::Variable {
                name: "a".into(),
                value_type: Some(ValueType::Number),
            }],
            body: Box::new(ASTNode::Block(vec![
                ASTNode::Assign {
                    name: "local_var".into(),
                    value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(10)))),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: true,
                },
                ASTNode::Return(Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Variable {
                        name: "a".into(),
                        value_type: Some(ValueType::Number),
                    }),
                    op: TokenKind::Plus,
                    right: Box::new(ASTNode::Variable {
                        name: "local_var".into(),
                        value_type: Some(ValueType::Number),
                    }),
                })),
            ])),
            return_type: ValueType::Number,
        };

        eval(ast_function, &mut env);

        // 関数呼び出し
        let ast_call = ASTNode::FunctionCall {
            name: "add_and_return".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![ASTNode::Literal(
                Value::Number(Fraction::from(5)),
            )])),
        };

        // 結果の確認
        let result = eval(ast_call, &mut env);
        assert_eq!(result, Value::Number(Fraction::from(15)));

        // スコープ外でローカル変数が見つからないことを確認
        let local_var_check = env.get(&"local_var".to_string(), None);
        assert!(local_var_check.is_none());
    }

    #[test]
    fn test_scope_and_global_variable() {
        let mut env = Env::new();

        // グローバル変数 z を定義
        let global_z = ASTNode::Assign {
            name: "z".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(3)))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        eval(global_z, &mut env);

        // f1 関数の定義
        let f1 = ASTNode::Function {
            name: "f1".to_string(),
            arguments: vec![
                ASTNode::Variable {
                    name: "x".into(),
                    value_type: Some(ValueType::Number),
                },
                ASTNode::Variable {
                    name: "y".into(),
                    value_type: Some(ValueType::Number),
                },
            ],
            body: Box::new(ASTNode::Block(vec![
                ASTNode::Assign {
                    name: "z".to_string(),
                    value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(2)))),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: false,
                },
                ASTNode::Assign {
                    name: "d".to_string(),
                    value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(3)))),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,

                    is_new: true,
                },
                ASTNode::Assign {
                    name: "z".to_string(),
                    value: Box::new(ASTNode::Assign {
                        name: "d".to_string(),
                        value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(4)))),
                        variable_type: EnvVariableType::Mutable,
                        value_type: ValueType::Number,
                        is_new: false,
                    }),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: false,
                },
                ASTNode::Return(Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::BinaryOp {
                        left: Box::new(ASTNode::Variable {
                            name: "x".into(),
                            value_type: Some(ValueType::Number),
                        }),
                        op: TokenKind::Plus,
                        right: Box::new(ASTNode::Variable {
                            name: "y".into(),
                            value_type: Some(ValueType::Number),
                        }),
                    }),
                    op: TokenKind::Plus,
                    right: Box::new(ASTNode::Variable {
                        name: "z".into(),
                        value_type: Some(ValueType::Number),
                    }),
                })),
            ])),
            return_type: ValueType::Number,
        };
        eval(f1, &mut env);

        // f2 関数の定義
        let f2 = ASTNode::Function {
            name: "f2".to_string(),
            arguments: vec![
                ASTNode::Variable {
                    name: "x".into(),
                    value_type: Some(ValueType::Number),
                },
                ASTNode::Variable {
                    name: "y".into(),
                    value_type: Some(ValueType::Number),
                },
            ],
            body: Box::new(ASTNode::Return(Box::new(ASTNode::BinaryOp {
                left: Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::Variable {
                        name: "x".into(),
                        value_type: Some(ValueType::Number),
                    }),
                    op: TokenKind::Plus,
                    right: Box::new(ASTNode::Variable {
                        name: "y".into(),
                        value_type: Some(ValueType::Number),
                    }),
                }),
                op: TokenKind::Plus,
                right: Box::new(ASTNode::Variable {
                    name: "z".into(),
                    value_type: Some(ValueType::Number),
                }),
            }))),
            return_type: ValueType::Number,
        };
        eval(f2, &mut env);

        // f3 関数の定義
        let f3 = ASTNode::Function {
            name: "f3".to_string(),
            arguments: vec![],
            body: Box::new(ASTNode::Return(Box::new(ASTNode::Literal(Value::Number(
                Fraction::from(1),
            ))))),
            return_type: ValueType::Number,
        };
        eval(f3, &mut env);

        // f1 の呼び出し
        let call_f1 = ASTNode::FunctionCall {
            name: "f1".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![
                ASTNode::Literal(Value::Number(Fraction::from(2))),
                ASTNode::Literal(Value::Number(Fraction::from(0))),
            ])),
        };
        let result_f1 = eval(call_f1, &mut env);
        assert_eq!(result_f1, Value::Number(Fraction::from(6))); // 2 + 0 + z(4) = 6

        // f2 の呼び出し (f1 の影響で z = 4)
        let call_f2 = ASTNode::FunctionCall {
            name: "f2".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![
                ASTNode::Literal(Value::Number(Fraction::from(2))),
                ASTNode::Literal(Value::Number(Fraction::from(0))),
            ])),
        };
        let result_f2 = eval(call_f2, &mut env);
        assert_eq!(result_f2, Value::Number(Fraction::from(6))); // 2 + 0 + z(4) = 6

        // f3 の呼び出し
        let call_f3 = ASTNode::FunctionCall {
            name: "f3".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![])),
        };
        let result_f3 = eval(call_f3, &mut env);
        assert_eq!(result_f3, Value::Number(Fraction::from(1)));
    }

    #[test]
    fn test_global_variable_and_functions() {
        let mut env = Env::new();

        // グローバル変数の定義
        let global_z = ASTNode::Assign {
            name: "z".to_string(),
            value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(3)))),
            variable_type: EnvVariableType::Mutable,
            value_type: ValueType::Number,
            is_new: true,
        };
        eval(global_z, &mut env);

        // f1関数の定義
        let f1 = ASTNode::Function {
            name: "f1".to_string(),
            arguments: vec![
                ASTNode::Variable {
                    name: "x".into(),
                    value_type: Some(ValueType::Number),
                },
                ASTNode::Variable {
                    name: "y".into(),
                    value_type: Some(ValueType::Number),
                },
            ],
            body: Box::new(ASTNode::Block(vec![
                ASTNode::Assign {
                    name: "z".to_string(),
                    value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(2)))),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: false,
                },
                ASTNode::Assign {
                    name: "d".to_string(),
                    value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(3)))),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: true,
                },
                ASTNode::Assign {
                    name: "z".to_string(),
                    value: Box::new(ASTNode::Assign {
                        name: "d".to_string(),
                        value: Box::new(ASTNode::Literal(Value::Number(Fraction::from(4)))),
                        variable_type: EnvVariableType::Mutable,
                        value_type: ValueType::Number,
                        is_new: false,
                    }),
                    variable_type: EnvVariableType::Mutable,
                    value_type: ValueType::Number,
                    is_new: false,
                },
                ASTNode::Return(Box::new(ASTNode::BinaryOp {
                    left: Box::new(ASTNode::BinaryOp {
                        left: Box::new(ASTNode::Variable {
                            name: "x".into(),
                            value_type: Some(ValueType::Number),
                        }),
                        op: TokenKind::Plus,
                        right: Box::new(ASTNode::Variable {
                            name: "y".into(),
                            value_type: Some(ValueType::Number),
                        }),
                    }),
                    op: TokenKind::Plus,
                    right: Box::new(ASTNode::Variable {
                        name: "z".into(),
                        value_type: Some(ValueType::Number),
                    }),
                })),
            ])),
            return_type: ValueType::Number,
        };
        eval(f1, &mut env);

        // f1の呼び出し
        let call_f1 = ASTNode::FunctionCall {
            name: "f1".to_string(),
            arguments: Box::new(ASTNode::FunctionCallArgs(vec![
                ASTNode::Literal(Value::Number(Fraction::from(2))),
                ASTNode::Literal(Value::Number(Fraction::from(0))),
            ])),
        };
        let result = eval(call_f1, &mut env);
        assert_eq!(result, Value::Number(Fraction::from(6))); // 2 + 0 + 4 = 6
    }

    #[test]
    fn test_comparison_operations() {
        let mut env = Env::new();
        let ast = ASTNode::Eq {
            left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1))))
        };
        assert_eq!(Value::Bool(true), eval(ast, &mut env));

        let ast = ASTNode::Gte {
            left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1))))
        };
        assert_eq!(Value::Bool(true), eval(ast, &mut env));

        let ast = ASTNode::Gt {
            left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1))))
        };
        assert_eq!(Value::Bool(false), eval(ast, &mut env));

        let ast = ASTNode::Lte {
            left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1))))
        };
        assert_eq!(Value::Bool(true), eval(ast, &mut env));

        let ast = ASTNode::Lt {
            left: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1)))),
            right: Box::new(ASTNode::Literal(Value::Number(Fraction::from(1))))
        };
        assert_eq!(Value::Bool(false), eval(ast, &mut env));
    }
}
