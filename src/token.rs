use fraction::Fraction;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Immutable,
    Mutable,
    Colon,
    Identifier(String),
    String(String),
    Number(Fraction),
    Void,
    Equal,
    Plus,
    Minus,
    Mul,
    Div,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Eof,
    Function,
    BackSlash,
    Pipe,
    Return,
    Comma,
    RArrow,
    Match,
    LBrancket,
    RBrancket,
    RRocket,
    If,
    Else,
    Eq,
    Lte,
    Lt,
    Gte,
    Gt,
    PublicStruct,
    PrivateStruct,
    Pub,
    Dot,
    Impl,
    CommentBlock(String),
    CommentLine(String),
}
