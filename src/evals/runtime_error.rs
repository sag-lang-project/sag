#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl RuntimeError {
    pub fn new(message: &str, line: usize, column: usize) -> Self {
        Self {
            message: message.to_string(),
            line,
            column,
        }
    }

    pub fn message_with_source(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let error_line = lines.get(self.line - 1).unwrap_or(&"");
        format!(
            "Runtime Error: {}\n --> line {}, column {}\n | {}\n | {}^",
            self.message,
            self.line,
            self.column,
            error_line,
            " ".repeat(self.column)
        )
    }
}
