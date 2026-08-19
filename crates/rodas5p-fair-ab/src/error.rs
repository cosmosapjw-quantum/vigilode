use thiserror::Error;

#[derive(Debug, Error)]
pub enum FairError {
    #[error(transparent)]
    Core(#[from] rodas5p_core::CoreError),
    #[error("invalid fair-comparison input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
}

pub type FairResult<T> = Result<T, FairError>;
