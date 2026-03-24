//! Compiler error types.

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Parse error at line {line}, col {col}: {message}")]
    Parse { message: String, line: usize, col: usize },

    #[error("Type error at line {line}, col {col}: {message}")]
    Type { message: String, line: usize, col: usize },

    #[error("Unsupported feature at line {line}, col {col}: {message}")]
    Unsupported { message: String, line: usize, col: usize },

    #[error("Internal compiler error: {0}")]
    Internal(String),
}

impl CompileError {
    pub fn line(&self) -> usize {
        match self {
            Self::Parse { line, .. } |
            Self::Type { line, .. } |
            Self::Unsupported { line, .. } => *line,
            Self::Internal(_) => 0,
        }
    }

    pub fn col(&self) -> usize {
        match self {
            Self::Parse { col, .. } |
            Self::Type { col, .. } |
            Self::Unsupported { col, .. } => *col,
            Self::Internal(_) => 0,
        }
    }
}
