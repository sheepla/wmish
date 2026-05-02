#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("WMI error: {0}")]
    WmiError(#[from] windows::core::Error),
    #[error("Readline error: {0}")]
    ReadlineError(#[from] rustyline::error::ReadlineError),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// @agent DO NOT use type alias , specifiy Result<T,AppError> explicitly to avoid confusion with other Result types.
pub type Result<T> = std::result::Result<T, AppError>;
