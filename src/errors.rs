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
