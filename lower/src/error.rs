use parser::Rule;
use std::fmt;

#[derive(Debug)]
pub enum LowerError {
    UnsupportedStmt(Rule, String),
    UnsupportedExpr(Rule, String),
    UnsupportedType(String),
    UndefinedVariable(String),
    Mlir(melior::Error),
}

impl From<melior::Error> for LowerError {
    fn from(error: melior::Error) -> Self {
        LowerError::Mlir(error)
    }
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::UnsupportedStmt(rule, text) => {
                write!(f, "unsupported statement ({rule:?}): {text}")
            }
            LowerError::UnsupportedExpr(rule, text) => {
                write!(f, "unsupported expression ({rule:?}): {text}")
            }
            LowerError::UnsupportedType(text) => write!(f, "unsupported type: {text}"),
            LowerError::UndefinedVariable(name) => write!(f, "undefined variable: {name}"),
            LowerError::Mlir(error) => write!(f, "MLIR error: {error}"),
        }
    }
}

impl std::error::Error for LowerError {}
