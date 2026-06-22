use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("command `{cmd}` failed (exit code {code})")]
    CommandFailed { cmd: String, code: i32 },
    #[error("command `{cmd}` not found on PATH")]
    CommandNotFound { cmd: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("download failed for `{what}` ({detail})")]
    Download { what: String, detail: String },
    #[error("{0}")]
    Other(String),
}

impl CoreError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
