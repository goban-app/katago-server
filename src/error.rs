use thiserror::Error;

#[derive(Error, Debug)]
pub enum KatagoError {
    #[error("Failed to start KataGo process: {0}")]
    ProcessStartFailed(String),

    #[error("KataGo process died unexpectedly")]
    ProcessDied,

    #[error("Command timeout after {0} seconds")]
    Timeout(u64),

    #[error("Failed to parse KataGo response: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[allow(dead_code)]
    #[error("Invalid GTP command: {0}")]
    InvalidCommand(String),

    #[allow(dead_code)]
    #[error("KataGo returned error: {0}")]
    ResponseError(String),
}

pub type Result<T> = std::result::Result<T, KatagoError>;
