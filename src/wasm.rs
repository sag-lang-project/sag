use crate::builtin::register_builtins;
use crate::environment::Env;
use crate::evals::evals;
use crate::parsers::Parser;
use crate::tokenizer::tokenize;
use crate::value::Value;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    pub(crate) static CONSOLE_OUTPUT: RefCell<String> = RefCell::new(String::new());
}

#[wasm_bindgen]
pub fn evaluate(input: &str) -> String {
    CONSOLE_OUTPUT.with(|output| output.borrow_mut().clear());

    let tokens = tokenize(&input.to_string());
    let mut env = Env::new();
    let builtins = register_builtins(&mut env);
    let mut parser = Parser::new(tokens, builtins.clone());
    let ast_nodes = parser.parse_lines();
    if let Err(ref e) = ast_nodes {
        let error_message = e.message_with_source(&input);
        return format!(
            "__ConsoleOutput__{}__Result__{}",
            error_message,
            Value::Void
        );
    }
    let result = evals(ast_nodes.unwrap(), &mut env);
    if let Err(ref e) = result {
        let error_message = e.message_with_source(&input);
        return format!(
            "__ConsoleOutput__{}__Result__{}",
            error_message,
            Value::Void
        );
    }

    let output = CONSOLE_OUTPUT.with(|output| output.borrow().clone());
    let result_str = format!("{}", result.unwrap().last().unwrap_or(&Value::Void));
    format!(
        "__ConsoleOutput__{}__Result__{}",
        output.trim_end(),
        result_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_basic_arithmetic() {
        let result = evaluate("1 + 2");
        assert_eq!(result, "__ConsoleOutput____Result__3");
    }

    #[test]
    fn test_evaluate_global_variable_and_functions() {
        let input = r#"
val mut z = 3

fun f1(x: number, y: number): number {
    z = 2
    val mut d = 3
    z = d = 4
    return x + y + z
}

|2, 0| -> f1
"#;
        let result = evaluate(input);
        assert_eq!(result, "__ConsoleOutput____Result__6");
    }

    #[test]
    fn test_evaluate_multiple_functions() {
        let input = r#"
val mut z = 3

fun f1(x: number, y: number): number {
    z = 2
    val mut d = 3
    z = d = 4
    return x + y + z
}

fun f2(x: number, y: number): number {
    return x + y + z
}

fun f3(): number {
    return 1
}

fun f4(): number {
    return 2 + 3 / 4
}

|2, 0| -> f1
|2, 0| -> f2
|| -> f3
|| -> f4
"#;
        let result = evaluate(input);
        assert_eq!(result, "__ConsoleOutput____Result__11/4");
    }
}
