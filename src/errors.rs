#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("WMI error: {0}")]
    WmiError(windows::core::Error),
    #[error("Readline error: {0}")]
    ReadlineError(rustyline::error::ReadlineError),
    #[error("Parse error: {0}")]
    ParseError(String),
}
