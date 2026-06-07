use crate::token::{Token, TokenKind};
use fraction::Fraction;

struct Tokenizer {
    tokens: Vec<Token>,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    nesting_count: usize,
}

impl Tokenizer {
    pub fn new(line: &String) -> Self {
        Tokenizer {
            pos: 0,
            column: 1,
            line: 1,
            chars: line.chars().collect(),
            tokens: vec![],
            nesting_count: 0,
        }
    }

    // Store the current position before advancing
    pub fn store_position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    pub fn get_position_char(&self, pos: usize) -> char {
        if pos >= self.chars.len() {
            return '\0';
        }
        self.chars[pos]
    }
}

fn is_space(c: &char) -> bool {
    *c == ' '
}

fn is_tab(c: &char) -> bool {
    *c == '\t'
}

fn is_digit(c: &char) -> bool {
    *c >= '0' && *c <= '9'
}

fn get_digit(tokenizer: &mut Tokenizer) -> Fraction {
    let mut num = String::new();
    let mut pos = tokenizer.pos;
    let mut is_decimal_point = false;
    loop {
        let c = tokenizer.get_position_char(pos);
        if is_digit(&c) {
            num += &c.to_string();
            pos += 1;
        } else if c == '.' {
            if is_decimal_point {
                break;
            }
            is_decimal_point = true;
            num += &c.to_string();
            pos += 1;
            if !is_digit(&tokenizer.get_position_char(pos)) {
                pos -= 1;
                break;
            }
        } else {
            break;
        }
    }
    tokenizer.pos = pos;
    tokenizer.column += num.len();
    match num.parse::<f64>() {
        Ok(n) => fraction::Fraction::from(n),
        Err(_) => fraction::Fraction::from(0),
    }
}

fn is_string(c: &char) -> bool {
    *c == '"'
}

fn get_identifier(tokenizer: &mut Tokenizer) -> String {
    let mut identifier = String::new();
    let mut pos = tokenizer.pos;
    loop {
        let c = tokenizer.get_position_char(pos);
        if c == '\0'
            || c == '\n'
            || c == ' '
            || c == ':'
            || c == ','
            || c == '('
            || c == ')'
            || c == '{'
            || c == '}'
            || c == '='
            || c == '+'
            || c == '-'
            || c == '*'
            || c == '/'
            || c == '%'
            || c == '.'
            || c == '|'
            || c == '<'
            || c == '>'
            || c == '\\'
            || c == '['
            || c == ']'
            || c == '\t'
        {
            break;
        }
        identifier += &c.to_string();
        pos += 1;
    }
    tokenizer.pos = pos;
    identifier
}

fn get_string(tokenizer: &mut Tokenizer) -> String {
    let mut str = String::new();
    let mut pos = tokenizer.pos + 1;
    loop {
        let c = tokenizer.get_position_char(pos);
        if c == '"' {
            pos += 1;
            tokenizer.pos = pos;
            break;
        }
        if c == '\0' {
            break;
        }
        str += &c.to_string();
        pos += 1;
    }
    tokenizer.column += str.len() + 2;
    str
}

fn is_line_break(c: &char) -> bool {
    *c == '\n'
}

fn is_colon(c: &char) -> bool {
    *c == ':'
}

fn is_comma(c: &char) -> bool {
    *c == ','
}

fn is_semicoron(c: &char) -> bool {
    *c == ';'
}

fn is_function_call_args(c: &char) -> bool {
    *c == '|'
}

fn is_line_comment(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "//".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_comment_block(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "```".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_import(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "import ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_from(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "from ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn get_line_comment_string(tokenizer: &mut Tokenizer) -> String {
    let mut comment = String::new();
    let mut pos = tokenizer.pos + 1;
    loop {
        let c = tokenizer.get_position_char(pos);
        if c == '\0' || c == '\n' {
            tokenizer.pos = pos;
            break;
        }
        comment += &c.to_string();
        pos += 1;
    }
    tokenizer.column += comment.len() + 2;
    comment
}

fn get_comment_string(tokenizer: &mut Tokenizer) -> String {
    let mut comment = String::new();
    let mut pos = tokenizer.pos + 3;
    let mut back_quote_count = 0;
    let mut before_c = '\0';
    loop {
        let c = tokenizer.get_position_char(pos);
        if c == '\0' {
            tokenizer.pos = pos;
            break;
        }
        if back_quote_count == 3 {
            pos += 2;
            tokenizer.pos = pos;
            break;
        }
        if c == '`' && back_quote_count == 0 {
            back_quote_count += 1;
            before_c = c;
            pos += 1;
            continue;
        }
        if c == '`' && before_c == '`' {
            back_quote_count += 1;
            continue;
        }
        back_quote_count = 0;
        before_c = c;
        comment += &c.to_string();
        pos += 1;
    }
    tokenizer.column += comment.len() + 6;
    comment
}

fn is_break(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "break".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_continue(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "continue".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_immutable(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "val ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_mutable(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "val mut ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_function(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "fun ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_return(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "return ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_exponent(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "**".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_match(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "match ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_right_arrow(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "->".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_right_rocket(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "=>".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_and(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in " and ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_or(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in " or ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_xor(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "xor".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_result(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Result".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_success(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Suc".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_failure(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Fail".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_option(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Option".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_some(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Some".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_none(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "None".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_void(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "Void".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_pub(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "pub ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_struct(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "struct ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_impl(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "impl ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_for(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "for ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_in(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "in ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_eq(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "==".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_lte(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "<=".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_lt(c: char) -> bool {
    c == '<'
}

fn is_gte(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in ">=".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_gt(c: char) -> bool {
    c == '>'
}

fn is_true(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "true".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_false(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "false".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_if(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "if ".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

fn is_else(tokenizer: &mut Tokenizer) -> bool {
    for (i, c) in "else".chars().enumerate() {
        if c != tokenizer.get_position_char(i + tokenizer.pos) {
            return false;
        }
    }
    true
}

pub fn tokenize(line: &String) -> Vec<Token> {
    let mut tokenizer = Tokenizer::new(&line);
    loop {
        let c = tokenizer.get_position_char(tokenizer.pos);
        if is_line_break(&c) || is_semicoron(&c) {
            let (line, column) = tokenizer.store_position();
            match tokenizer.tokens.last() {
                Some(Token { kind, .. }) => {
                    if kind != &TokenKind::Eof {
                        tokenizer.tokens.push(Token {
                            kind: TokenKind::Eof,
                            line,
                            column,
                        });
                    }
                }
                _ => tokenizer.tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    column,
                }),
            }
            tokenizer.line += 1;
            tokenizer.pos += 1;
            tokenizer.column = 1;
            continue;
        }
        if c == '\0' {
            break;
        }
        if is_space(&c) {
            tokenizer.pos += 1;
            tokenizer.column += 1;
            continue;
        }
        if is_tab(&c) {
            tokenizer.pos += 1;
            tokenizer.column += 1;
            continue;
        }
        if is_digit(&c) {
            let (line, column) = tokenizer.store_position();
            let num = get_digit(&mut tokenizer);
            tokenizer.tokens.push(Token {
                kind: TokenKind::Number(num),
                line,
                column,
            });
            continue;
        }

        if is_string(&c) {
            let (line, column) = tokenizer.store_position();
            let str = get_string(&mut tokenizer);
            tokenizer.tokens.push(Token {
                kind: TokenKind::String(str),
                line,
                column,
            });
            continue;
        }

        if is_break(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 5;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Break,
                line,
                column,
            });
            tokenizer.pos += 5;
            continue;
        }

        if is_continue(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 8;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Continue,
                line,
                column,
            });
            tokenizer.pos += 8;
            continue;
        }

        if is_mutable(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 8;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Mutable,
                line,
                column,
            });
            tokenizer.pos += 8;
            continue;
        }

        if is_immutable(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Immutable,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_line_comment(&mut tokenizer) {
            let _comment = get_line_comment_string(&mut tokenizer);
            //tokenizer.tokens.push(Token::CommentLine(comment));
            tokenizer.pos += 1;
            tokenizer.column += 1;
            continue;
        }

        if is_comment_block(&mut tokenizer) {
            let _comment = get_comment_string(&mut tokenizer);
            //tokenizer.tokens.push(Token::CommentBlock(comment));
            continue;
        }

        if is_function(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Function,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_import(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 7;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Import,
                line,
                column,
            });
            tokenizer.pos += 7;
            continue;
        }

        if is_from(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 5;
            tokenizer.tokens.push(Token {
                kind: TokenKind::From,
                line,
                column,
            });
            tokenizer.pos += 5;
            continue;
        }

        if is_exponent(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Pow,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_match(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 6;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Match,
                line,
                column,
            });
            tokenizer.pos += 6;
            continue;
        }

        if is_return(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 7;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Return,
                line,
                column,
            });
            tokenizer.pos += 7;
            continue;
        }

        if is_right_arrow(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::RArrow,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_struct(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 6;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Struct,
                line,
                column,
            });
            tokenizer.pos += 6;
            continue;
        }

        if is_impl(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Impl,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_pub(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Pub,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_option(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 6;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Option,
                line,
                column,
            });
            tokenizer.pos += 6;
            continue;
        }

        if is_some(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Some,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_none(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::None,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_void(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Void,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_and(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 5;
            tokenizer.tokens.push(Token {
                kind: TokenKind::And,
                line,
                column,
            });
            tokenizer.pos += 5;
            continue;
        }
        if is_or(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Or,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_xor(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Xor,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_result(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 6;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Result,
                line,
                column,
            });
            tokenizer.pos += 6;
            continue;
        }

        if is_success(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Success,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_failure(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Failure,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_for(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 3;
            tokenizer.tokens.push(Token {
                kind: TokenKind::For,
                line,
                column,
            });
            tokenizer.pos += 3;
            continue;
        }

        if is_in(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::In,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_right_rocket(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::RRocket,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_if(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::If,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_else(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Else,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_eq(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Eq,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_lte(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Lte,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_lt(c) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 1;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Lt,
                line,
                column,
            });
            tokenizer.pos += 1;
            continue;
        }

        if is_gte(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 2;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Gte,
                line,
                column,
            });
            tokenizer.pos += 2;
            continue;
        }

        if is_gt(c) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 1;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Gt,
                line,
                column,
            });
            tokenizer.pos += 1;
            continue;
        }

        if is_true(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 4;
            tokenizer.tokens.push(Token {
                kind: TokenKind::True,
                line,
                column,
            });
            tokenizer.pos += 4;
            continue;
        }

        if is_false(&mut tokenizer) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 5;
            tokenizer.tokens.push(Token {
                kind: TokenKind::False,
                line,
                column,
            });
            tokenizer.pos += 5;
            continue;
        }

        if is_colon(&c) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 1;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Colon,
                line,
                column,
            });
            tokenizer.pos += 1;
            continue;
        }

        if is_comma(&c) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 1;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Comma,
                line,
                column,
            });
            tokenizer.pos += 1;
            continue;
        }

        if is_function_call_args(&c) {
            let (line, column) = tokenizer.store_position();
            tokenizer.column += 1;
            tokenizer.tokens.push(Token {
                kind: TokenKind::Pipe,
                line,
                column,
            });
            tokenizer.pos += 1;
            continue;
        }

        let (line, column) = tokenizer.store_position();
        tokenizer.column += 1;
        match c {
            '+' => tokenizer.tokens.push(Token {
                kind: TokenKind::Plus,
                line,
                column,
            }),
            '-' => tokenizer.tokens.push(Token {
                kind: TokenKind::Minus,
                line,
                column,
            }),
            '*' => tokenizer.tokens.push(Token {
                kind: TokenKind::Mul,
                line,
                column,
            }),
            '/' => tokenizer.tokens.push(Token {
                kind: TokenKind::Div,
                line,
                column,
            }),
            '%' => tokenizer.tokens.push(Token {
                kind: TokenKind::Mod,
                line,
                column,
            }),
            '(' => tokenizer.tokens.push(Token {
                kind: TokenKind::LParen,
                line,
                column,
            }),
            ')' => tokenizer.tokens.push(Token {
                kind: TokenKind::RParen,
                line,
                column,
            }),
            '[' => tokenizer.tokens.push(Token {
                kind: TokenKind::LBrancket,
                line,
                column,
            }),
            ']' => tokenizer.tokens.push(Token {
                kind: TokenKind::RBrancket,
                line,
                column,
            }),
            '.' => tokenizer.tokens.push(Token {
                kind: TokenKind::Dot,
                line,
                column,
            }),
            '\\' => tokenizer.tokens.push(Token {
                kind: TokenKind::BackSlash,
                line,
                column,
            }),
            '{' => {
                tokenizer.nesting_count += 1;
                tokenizer.tokens.push(Token {
                    kind: TokenKind::LBrace,
                    line,
                    column,
                });
            }
            '}' => {
                tokenizer.nesting_count -= 1;
                tokenizer.tokens.push(Token {
                    kind: TokenKind::RBrace,
                    line,
                    column,
                });
                if tokenizer.nesting_count == 0 {
                    tokenizer.tokens.push(Token {
                        kind: TokenKind::Eof,
                        line,
                        column,
                    });
                }
            }
            '=' => tokenizer.tokens.push(Token {
                kind: TokenKind::Equal,
                line,
                column,
            }),
            _ => {
                let (line, column) = tokenizer.store_position();
                let value = get_identifier(&mut tokenizer);
                tokenizer.column += value.len() - 1;
                tokenizer.tokens.push(Token {
                    kind: TokenKind::Identifier(value),
                    line,
                    column,
                });
                continue;
            }
        }
        tokenizer.pos += 1;
    }
    let (line, column) = tokenizer.store_position();
    match tokenizer.tokens.last() {
        Some(Token { kind, .. }) => {
            if kind != &TokenKind::Eof {
                tokenizer.tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    column,
                });
            }
        }
        _ => tokenizer.tokens.push(Token {
            kind: TokenKind::Eof,
            line,
            column,
        }),
    }
    tokenizer.tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_four_basic_arithmetic_operations() {
        let result = vec![
            TokenKind::Minus,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Plus,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Mul,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Div,
            TokenKind::Number(Fraction::from(4)),
            TokenKind::Mod,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"-1 + 2 * 3/4 % 3".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }
    #[test]
    fn test_variable_definition() {
        let result = vec![
            TokenKind::Mutable,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val mut x = 1".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![
            TokenKind::Immutable,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Identifier("num".into()),
            TokenKind::Equal,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val x: num = 1".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_multiline() {
        let result = vec![
            TokenKind::Minus,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Plus,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Eof,
            TokenKind::Immutable,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"-1 + 2\n val x = 1".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_string() {
        let result = vec![TokenKind::String("Hello World!!".into()), TokenKind::Eof];
        for (i, token) in tokenize(&"\"Hello World!!\"".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_function() {
        let result = vec![
            TokenKind::Function,
            TokenKind::Identifier("foo".into()),
            TokenKind::Equal,
            TokenKind::LParen,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Comma,
            TokenKind::Identifier("y".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::RParen,
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Identifier("x".into()),
            TokenKind::Plus,
            TokenKind::Identifier("y".into()),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in
            tokenize(&"fun foo = (x:number, y: number): number {\n return x + y \n}".to_string())
                .into_iter()
                .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }
    #[test]
    fn test_call_function() {
        let result = vec![
            TokenKind::LParen,
            TokenKind::Identifier("x".into()),
            TokenKind::Comma,
            TokenKind::Identifier("y".into()),
            TokenKind::RParen,
            TokenKind::RArrow,
            TokenKind::Identifier("foo".into()),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"(x, y) -> foo".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_decimal_point() {
        let result = vec![TokenKind::Number(Fraction::from(1.5)), TokenKind::Eof];
        for (i, token) in tokenize(&"1.5".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_list() {
        let result = vec![
            TokenKind::LBrancket,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Comma,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Comma,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::RBrancket,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"[1, 2, 3]".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![
            TokenKind::LBrancket,
            TokenKind::String("Hello".into()),
            TokenKind::Comma,
            TokenKind::String("World".into()),
            TokenKind::RBrancket,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"[\"Hello\", \"World\"]".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_call_functions() {
        let result = vec![
            TokenKind::Number(Fraction::from(1)),
            TokenKind::RArrow,
            TokenKind::Identifier("f1".into()),
            TokenKind::RArrow,
            TokenKind::Identifier("f2".into()),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"1 -> f1 -> f2".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_lambda() {
        let result = vec![
            TokenKind::Immutable,
            TokenKind::Identifier("inc".into()),
            TokenKind::Equal,
            TokenKind::BackSlash,
            TokenKind::Pipe,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Pipe,
            TokenKind::RRocket,
            TokenKind::Identifier("x".into()),
            TokenKind::Plus,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val inc = \\|x: number| => x + 1".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_if() {
        let result = vec![
            TokenKind::If,
            TokenKind::Identifier("x".into()),
            TokenKind::Eq,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"if x == 1 {\n return 1\n }".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_else() {
        let result = vec![
            TokenKind::If,
            TokenKind::Identifier("x".into()),
            TokenKind::Eq,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(0)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"if x == 1 {\n return 1\n } else {\n return 0 \n}".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_else_if() {
        let result = vec![
            TokenKind::If,
            TokenKind::Identifier("x".into()),
            TokenKind::Eq,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
            TokenKind::Else,
            TokenKind::If,
            TokenKind::Identifier("x".into()),
            TokenKind::Eq,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Return,
            TokenKind::Number(Fraction::from(0)),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(
            &"if x == 1 {\n return 1\n } else if x == 2 {\n return 2 \n} else {\n return 0 \n}"
                .to_string(),
        )
        .into_iter()
        .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_funtion_call_front() {
        let result = vec![
            TokenKind::Identifier("f1".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"f1()".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_comparison_operations() {
        let result = vec![
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eq,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"1 == 1".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }

        let result = vec![
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Gt,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];

        for (i, token) in tokenize(&"2 > 1".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }

        let result = vec![
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Gte,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"3 >= 3".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }

        let result = vec![
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Lt,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"1 < 2".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }

        let result = vec![
            TokenKind::Number(Fraction::from(4)),
            TokenKind::Lte,
            TokenKind::Number(Fraction::from(4)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"4 <= 4".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_struct() {
        let result = vec![
            TokenKind::Struct,
            TokenKind::Identifier("Point".into()),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Comma,
            TokenKind::Eof,
            TokenKind::Identifier("y".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"struct Point {\n x: number,\n y: number\n }".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![
            TokenKind::Pub,
            TokenKind::Struct,
            TokenKind::Identifier("Point".into()),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Pub,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Comma,
            TokenKind::Eof,
            TokenKind::Identifier("y".into()),
            TokenKind::Colon,
            TokenKind::Identifier("number".into()),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in
            tokenize(&"pub struct Point {\n pub x: number,\n y: number\n }".to_string())
                .into_iter()
                .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }
    #[test]
    fn test_struct_instance() {
        let result = vec![
            TokenKind::Identifier("Point".into()),
            TokenKind::LBrace,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Comma,
            TokenKind::Identifier("y".into()),
            TokenKind::Colon,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"Point { x: 1, y: 2 }".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_assign_struct() {
        let result = vec![
            TokenKind::Immutable,
            TokenKind::Identifier("point".into()),
            TokenKind::Equal,
            TokenKind::Identifier("Point".into()),
            TokenKind::LBrace,
            TokenKind::Identifier("x".into()),
            TokenKind::Colon,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Comma,
            TokenKind::Identifier("y".into()),
            TokenKind::Colon,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val point = Point { x: 1, y: 2 }".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_struct_field_access() {
        let result = vec![
            TokenKind::Identifier("point".into()),
            TokenKind::Dot,
            TokenKind::Identifier("x".into()),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"point.x".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_impl() {
        let result = vec![
            TokenKind::Impl,
            TokenKind::Identifier("Point".into()),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Function,
            TokenKind::Identifier("x".into()),
            TokenKind::Equal,
            TokenKind::LParen,
            TokenKind::Identifier("self".into()),
            TokenKind::Colon,
            TokenKind::Identifier("Point".into()),
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Identifier("self".into()),
            TokenKind::Dot,
            TokenKind::Identifier("x".into()),
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        for (i, token) in
            tokenize(&"impl Point {\n fun x = (self: Point) {\n self.x\n }\n }".to_string())
                .into_iter()
                .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_comment_block() {
        assert_eq!(
            tokenize(&"```# Title\n## title1```".to_string())[0].kind,
            TokenKind::Eof
        );
    }

    #[test]
    fn test_commnet_line() {
        assert_eq!(tokenize(&"// comment".to_string())[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_add_tab() {
        let result = vec![
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Plus,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Plus,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"1\t+ 2\t+ 3".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_identifier() {
        let result = vec![
            TokenKind::Identifier("x".into()),
            TokenKind::LBrancket,
            TokenKind::RBrancket,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"x[]".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_for() {
        let result = vec![
            TokenKind::For,
            TokenKind::Identifier("x".into()),
            TokenKind::In,
            TokenKind::LBrancket,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Comma,
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Comma,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::RBrancket,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"for x in [1, 2, 3]".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_import() {
        let token_kinds = vec![
            TokenKind::Import,
            TokenKind::Identifier("foo1".into()),
            TokenKind::Comma,
            TokenKind::Identifier("foo2".into()),
            TokenKind::Comma,
            TokenKind::Identifier("foo3".into()),
            TokenKind::From,
            TokenKind::Identifier("Foo".into()),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"import foo1,foo2, foo3 from Foo".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, token_kinds[i]);
        }
    }

    #[test]
    fn test_export() {
        let result = vec![
            TokenKind::Pub,
            TokenKind::Identifier("foo1".into()),
            TokenKind::Equal,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"pub foo1 = 1".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_convert_number() {
        let result = vec![
            TokenKind::Number(Fraction::from(1.2)),
            TokenKind::Dot,
            TokenKind::Identifier("a".into()),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"1.2.a".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_option() {
        let result = vec![
            TokenKind::Mutable,
            TokenKind::Identifier("foo".into()),
            TokenKind::Colon,
            TokenKind::Option,
            TokenKind::Lt,
            TokenKind::Identifier("number".into()),
            TokenKind::Gt,
            TokenKind::Equal,
            TokenKind::Some,
            TokenKind::LParen,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::RParen,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val mut foo: Option<number> = Some(1)".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![
            TokenKind::Mutable,
            TokenKind::Identifier("foo".into()),
            TokenKind::Colon,
            TokenKind::Option,
            TokenKind::Lt,
            TokenKind::Identifier("number".into()),
            TokenKind::Gt,
            TokenKind::Equal,
            TokenKind::None,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val mut foo: Option<number> = None".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_result() {
        let result = vec![
            TokenKind::Mutable,
            TokenKind::Identifier("foo".into()),
            TokenKind::Colon,
            TokenKind::Result,
            TokenKind::Lt,
            TokenKind::Identifier("number".into()),
            TokenKind::Comma,
            TokenKind::Identifier("string".into()),
            TokenKind::Gt,
            TokenKind::Equal,
            TokenKind::Success,
            TokenKind::LParen,
            TokenKind::Number(Fraction::from(1)),
            TokenKind::RParen,
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"val mut foo: Result<number, string> = Suc(1)".to_string())
            .into_iter()
            .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![
            TokenKind::Mutable,
            TokenKind::Identifier("foo".into()),
            TokenKind::Colon,
            TokenKind::Result,
            TokenKind::Lt,
            TokenKind::Identifier("number".into()),
            TokenKind::Comma,
            TokenKind::Identifier("string".into()),
            TokenKind::Gt,
            TokenKind::Equal,
            TokenKind::Failure,
            TokenKind::LParen,
            TokenKind::String("fail".into()),
            TokenKind::RParen,
            TokenKind::Eof,
        ];
        for (i, token) in
            tokenize(&"val mut foo: Result<number, string> = Fail(\"fail\")".to_string())
                .into_iter()
                .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_true_false() {
        let result = vec![TokenKind::True, TokenKind::Eof];
        for (i, token) in tokenize(&"true".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
        let result = vec![TokenKind::False, TokenKind::Eof];
        for (i, token) in tokenize(&"false".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }

    #[test]
    fn test_for_in_function() {
        let result = vec![
            TokenKind::Function,
            TokenKind::Identifier("f".into()),
            TokenKind::LParen,
            TokenKind::Identifier("xs".into()),
            TokenKind::Colon,
            TokenKind::Identifier("List".into()),
            TokenKind::Lt,
            TokenKind::Identifier("number".into()),
            TokenKind::Gt,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::For,
            TokenKind::Identifier("x".into()),
            TokenKind::In,
            TokenKind::Identifier("xs".into()),
            TokenKind::LBrace,
            TokenKind::Eof,
            TokenKind::Identifier("print".into()),
            TokenKind::LParen,
            TokenKind::Identifier("x".into()),
            TokenKind::RParen,
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];

        for (i, token) in
            tokenize(&"fun f(xs: List<number>) {\n for x in xs {\n print(x)\n }\n }".to_string())
                .into_iter()
                .enumerate()
        {
            assert_eq!(token.kind, result[i]);
        }
    }
    #[test]
    fn test_exponential() {
        let result = vec![
            TokenKind::Number(Fraction::from(2)),
            TokenKind::Pow,
            TokenKind::Number(Fraction::from(3)),
            TokenKind::Eof,
        ];
        for (i, token) in tokenize(&"2 ** 3".to_string()).into_iter().enumerate() {
            assert_eq!(token.kind, result[i]);
        }
    }
}
