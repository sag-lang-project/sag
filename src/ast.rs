use crate::environment::{EnvVariableType, ValueType};
use crate::token::TokenKind;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    // 数値や文字列などのリテラル
    Literal {
        value: Value,
        line: usize,
        column: usize,
    },
    // 変数
    Variable {
        name: String,
        value_type: Option<ValueType>,
        line: usize,
        column: usize,
    },
    Block {
        nodes: Vec<ASTNode>,
        line: usize,
        column: usize,
    },
    // -5, !trueなどの一つのオペランドを持つ演算子
    PrefixOp {
        op: TokenKind,
        expr: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    // 1 + 2のような二項演算子
    BinaryOp {
        left: Box<ASTNode>,
        op: TokenKind,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    // 変数の代入
    Assign {
        name: String,
        value: Box<ASTNode>,
        variable_type: EnvVariableType,
        value_type: ValueType,
        is_new: bool,
        line: usize,
        column: usize,
    },
    Function {
        name: String,
        arguments: Vec<ASTNode>,
        body: Box<ASTNode>,
        return_type: ValueType,
        line: usize,
        column: usize,
    },
    Method {
        name: String,
        arguments: Vec<ASTNode>,
        body: Box<ASTNode>,
        return_type: ValueType,
        is_mut: bool,
        line: usize,
        column: usize,
    },
    MethodCall {
        method_name: String,
        caller: Box<ASTNode>,
        arguments: Box<ASTNode>,
        builtin: bool,
        line: usize,
        column: usize,
    },
    FunctionCall {
        name: String,
        arguments: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    FunctionCallArgs {
        args: Vec<ASTNode>,
        line: usize,
        column: usize,
    },
    Return {
        expr: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Break {
        line: usize,
        column: usize,
    },
    Continue {
        line: usize,
        column: usize,
    },
    Lambda {
        arguments: Vec<ASTNode>,
        body: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    LambdaCall {
        lambda: Box<ASTNode>,
        arguments: Vec<ASTNode>,
        line: usize,
        column: usize,
    },
    Eq {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Gte {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Gt {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Lte {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Lt {
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    If {
        condition: Box<ASTNode>,
        is_statement: bool,
        then: Box<ASTNode>,
        else_: Option<Box<ASTNode>>,
        value_type: ValueType,
        line: usize,
        column: usize,
    },
    Struct {
        name: String,
        fields: HashMap<String, ASTNode>, // field_name: StructField
        line: usize,
        column: usize,
    },
    StructField {
        value_type: ValueType,
        is_public: bool,
        line: usize,
        column: usize,
    },
    StructFieldAccess {
        instance: Box<ASTNode>, // StructInstance, variable
        field_name: String,
        line: usize,
        column: usize,
    },
    StructFieldAssign {
        instance: Box<ASTNode>, // StructInstance, variable
        field_name: String,
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    StructInstance {
        name: String,
        fields: HashMap<String, ASTNode>,
        line: usize,
        column: usize,
    },
    Impl {
        base_struct: Box<ValueType>,
        methods: Vec<ASTNode>,
        line: usize,
        column: usize,
    },
    CommentBlock {
        comment: String,
        line: usize,
        column: usize,
    },
    For {
        variable: String,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Import {
        module_name: String,
        symbols: Vec<String>,
        line: usize,
        column: usize,
    },
    Public {
        node: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    OptionSome {
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    OptionNone {
        line: usize,
        column: usize,
    },
    ResultSuccess {
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    ResultFailure {
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    Match {
        expression: Box<ASTNode>,
        cases: Vec<(ASTNode, ASTNode)>,
        line: usize,
        column: usize,
    },
    DictKeyAccess {
        dict: Box<ASTNode>,
        key: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    DictAssign {
        dict: Box<ASTNode>,
        key: Box<ASTNode>,
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    ListIndexAccess {
        list: Box<ASTNode>,
        index: Box<ASTNode>,
        line: usize,
        column: usize,
    },
    ListIndexAssign {
        list: Box<ASTNode>,
        index: Box<ASTNode>,
        value: Box<ASTNode>,
        line: usize,
        column: usize,
    },
}
