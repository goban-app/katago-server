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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_start_failed_error() {
        let error = KatagoError::ProcessStartFailed("test error".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to start KataGo process: test error"
        );
    }

    #[test]
    fn test_process_died_error() {
        let error = KatagoError::ProcessDied;
        assert_eq!(error.to_string(), "KataGo process died unexpectedly");
    }

    #[test]
    fn test_timeout_error() {
        let error = KatagoError::Timeout(30);
        assert_eq!(error.to_string(), "Command timeout after 30 seconds");
    }

    #[test]
    fn test_parse_error() {
        let error = KatagoError::ParseError("invalid json".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to parse KataGo response: invalid json"
        );
    }

    #[test]
    fn test_invalid_command_error() {
        let error = KatagoError::InvalidCommand("bad command".to_string());
        assert_eq!(error.to_string(), "Invalid GTP command: bad command");
    }

    #[test]
    fn test_response_error() {
        let error = KatagoError::ResponseError("error message".to_string());
        assert_eq!(error.to_string(), "KataGo returned error: error message");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: KatagoError = io_error.into();
        assert!(error.to_string().contains("file not found"));
    }
}
