use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("dimension mismatch: {0}")]
    Dimension(String),
    #[error("invalid numerical input: {0}")]
    InvalidInput(String),
    #[error("non-finite numerical value: {0}")]
    NonFinite(String),
    #[error("linear solve failed: {0}")]
    LinearSolve(String),
    #[error("nonlinear solve failed: {0}")]
    NonlinearSolve(String),
    #[error("coefficient snapshot error: {0}")]
    Coefficients(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
