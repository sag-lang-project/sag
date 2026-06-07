use crate::token::Token;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    pub fn new(message: &str, token: &Token) -> Self {
        Self {
            message: message.to_string(),
            line: token.line,
            column: token.column,
        }
    }

    pub fn message_with_source(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let error_line = lines.get(self.line - 1).unwrap_or(&"");
        format!(
            "Parse Error: {}\n --> line {}, column {}\n | {}\n | {}^",
            self.message,
            self.line,
            self.column,
            error_line,
            " ".repeat(self.column)
        )
    }
}
