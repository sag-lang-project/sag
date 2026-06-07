use crate::ast::ASTNode;
use crate::environment::Env;
use crate::evals::eval;
use crate::evals::runtime_error::RuntimeError;
use crate::token::TokenKind;
use crate::value::Value;
use fraction::Fraction;

pub fn binary_op(
    op: TokenKind,
    left: Box<ASTNode>,
    right: Box<ASTNode>,
    line: usize,
    column: usize,
    env: &mut Env,
) -> Result<Value, RuntimeError> {
    let left_val = eval(*left, env)?;
    let right_val = eval(*right, env)?;
    let left_num = match &left_val {
        Value::Int(v) => Some(Fraction::from(*v)),
        Value::Number(v) => Some(v.clone()),
        _ => None,
    };
    let right_num = match &right_val {
        Value::Int(v) => Some(Fraction::from(*v)),
        Value::Number(v) => Some(v.clone()),
        _ => None,
    };

    match (&left_val, &right_val, &op) {
        (Value::String(l), Value::String(r), TokenKind::Plus) => {
            Ok(Value::String(format!("{}{}", l, r)))
        }
        (Value::String(l), r, TokenKind::Plus) => Ok(Value::String(format!("{}{}", l, r))),
        (Value::Int(l), Value::Int(r), TokenKind::Plus) => Ok(Value::Int(l + r)),
        (Value::Int(l), Value::Int(r), TokenKind::Minus) => Ok(Value::Int(l - r)),
        (Value::Int(l), Value::Int(r), TokenKind::Mul) => Ok(Value::Int(l * r)),
        (Value::Int(l), Value::Int(r), TokenKind::Div) => {
            Ok(Value::from_fraction((fraction::Fraction::from(*l)) / (fraction::Fraction::from(*r))))
        }
        (Value::Int(l), Value::Int(r), TokenKind::Mod) => Ok(Value::Int(l % r)),
        (Value::Int(l), Value::Int(r), TokenKind::Pow) if *r >= 0 => Ok(Value::Int(l.wrapping_pow(*r as u32))),
        (Value::Int(l), Value::Int(r), TokenKind::And) => Ok(Value::Int(l & r)),
        (Value::Int(l), Value::Int(r), TokenKind::Or) => Ok(Value::Int(l | r)),
        (Value::Int(l), Value::Int(r), TokenKind::Xor) => Ok(Value::Int(l ^ r)),
        _ if left_num.is_some() && right_num.is_some() && matches!(op, TokenKind::Plus) => {
            Ok(Value::from_fraction(left_num.unwrap() + right_num.unwrap()))
        }
        _ if left_num.is_some() && right_num.is_some() && matches!(op, TokenKind::Minus) => {
            Ok(Value::from_fraction(left_num.unwrap() - right_num.unwrap()))
        }
        _ if left_num.is_some() && right_num.is_some() && matches!(op, TokenKind::Mul) => {
            Ok(Value::from_fraction(left_num.unwrap() * right_num.unwrap()))
        }
        _ if left_num.is_some() && right_num.is_some() && matches!(op, TokenKind::Div) => {
            Ok(Value::from_fraction(left_num.unwrap() / right_num.unwrap()))
        }
        _ if left_num.is_some() && right_num.is_some() && matches!(op, TokenKind::Mod) => {
            Ok(Value::from_fraction(left_num.unwrap() % right_num.unwrap()))
        }
        (Value::Number(l), Value::Number(r), TokenKind::Minus) => Ok(Value::from_fraction(l - r)),
        (Value::Number(l), Value::Number(r), TokenKind::Mul) => Ok(Value::from_fraction(l * r)),
        (Value::Number(l), Value::Number(r), TokenKind::Div) => Ok(Value::from_fraction(l / r)),
        (Value::Number(l), Value::Number(r), TokenKind::Mod) => Ok(Value::from_fraction(l % r)),
        (Value::Number(l), Value::Number(r), TokenKind::Pow) => {
            let a = l.numer().unwrap();
            let b = l.denom().unwrap();
            let c = r.numer().unwrap();

            let raw_numer = a.wrapping_pow(*c as u32);
            let raw_denom = b.wrapping_pow(*c as u32);
            if raw_denom == 0 {
                return Err(RuntimeError::new("Division by zero", line, column));
            }
            Ok(Value::from_fraction((raw_numer, raw_denom).into()))
        }
        (Value::Bool(l), Value::Bool(r), TokenKind::And) => Ok(Value::Bool(*l && *r)),
        (Value::Bool(l), Value::Bool(r), TokenKind::Or) => Ok(Value::Bool(*l || *r)),
        (Value::Bool(l), Value::Bool(r), TokenKind::Xor) => Ok(Value::Bool(*l && !*r || !*l && *r)),
        (Value::Number(l), Value::Number(r), TokenKind::And) => Ok(Value::from_fraction(
            (
                l.numer().unwrap() & r.numer().unwrap(),
                l.denom().unwrap() & r.denom().unwrap(),
            )
            .into(),
        )),
        (Value::Number(l), Value::Number(r), TokenKind::Or) => Ok(Value::from_fraction(
            (
                l.numer().unwrap() | r.numer().unwrap(),
                l.denom().unwrap() | r.denom().unwrap(),
            )
            .into(),
        )),
        (Value::Number(l), Value::Number(r), TokenKind::Xor) => {
            // 分母を揃えて計算
            let a = l.numer().unwrap();
            let b = l.denom().unwrap();
            let c = r.numer().unwrap();
            let d = r.denom().unwrap();
            let ad =
                a.checked_mul(*d)
                    .ok_or(RuntimeError::new("Overflow Numerator", line, column))?;
            let cb =
                c.checked_mul(*b)
                    .ok_or(RuntimeError::new("Overflow Numerator", line, column))?;
            let raw_numer = ad ^ cb;
            let raw_denom =
                b.checked_mul(*d)
                    .ok_or(RuntimeError::new("Overflow Denominator", line, column))?;
            if raw_denom == 0 {
                return Err(RuntimeError::new("Division by zero", line, column));
            }
            Ok(Value::from_fraction((raw_numer, raw_denom).into()))
        }
        _ => Err(RuntimeError::new(
            format!(
                "Unsupported operation: {:?} {:?} {:?}",
                left_val, op, right_val
            )
            .as_str(),
            line,
            column,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtins;
    use crate::evals::evals;
    use crate::parsers::Parser;
    use crate::tokenizer::tokenize;

    #[test]
    fn add() {
        let mut env = Env::new();
        let input = "1 + 1".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[0], Value::Number(2.into()));
    }

    #[test]
    fn sub() {
        let mut env = Env::new();
        let input = "1 - 1".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[0], Value::Number(0.into()));
    }

    #[test]
    fn mul() {
        let mut env = Env::new();
        let input = "2 * 3".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[0], Value::Number(6.into()));
    }

    #[test]
    fn div() {
        let mut env = Env::new();
        let input = "2 / 3".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[0], Value::Number((2, 3).into()));
    }

    #[test]
    fn and() {
        let mut env = Env::new();
        for xy in [(true, true), (true, false), (false, true), (false, false)] {
            let input = format!("{} and {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Bool(xy.0 && xy.1));
        }

        for xy in [(1, 1), (1, 0), (0, 1), (0, 0)] {
            let input = format!("{} and {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Number((xy.0 & xy.1, 1).into()));
        }
    }

    #[test]
    fn or() {
        let mut env = Env::new();
        for xy in [(true, true), (true, false), (false, true), (false, false)] {
            let input = format!("{} or {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Bool(xy.0 || xy.1));
        }
        for xy in [(1, 1), (1, 0), (0, 1), (0, 0)] {
            let input = format!("{} or {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Number((xy.0 | xy.1, 1).into()));
        }
    }

    #[test]
    fn xor() {
        let mut env = Env::new();
        for xy in [(true, true), (true, false), (false, true), (false, false)] {
            let input = format!("{} xor {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Bool(xy.0 ^ xy.1));
        }
        for xy in [(1, 1), (1, 0), (0, 1), (0, 0)] {
            let input = format!("{} xor {}", xy.0, xy.1);
            let tokens = tokenize(&input);
            let mut parser = Parser::new(tokens, register_builtins(&mut env));
            let ast = parser.parse_lines();
            let result = evals(ast.unwrap(), &mut env).unwrap();
            assert_eq!(result[0], Value::Number((xy.0 ^ xy.1, 1).into()));
        }
    }

    #[test]
    fn pow() {
        let mut env = Env::new();
        let input = "val h = 188\nval w = 104\n w / (h / 100) ** 2".to_string();
        let tokens = tokenize(&input);
        let mut parser = Parser::new(tokens, register_builtins(&mut env));
        let ast = parser.parse_lines();
        let result = evals(ast.unwrap(), &mut env).unwrap();
        assert_eq!(result[2], Value::Number((65000, 2209).into()));
    }
}
