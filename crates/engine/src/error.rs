use std::fmt;

/// Engine result type
pub type EngineResult<T> = std::result::Result<T, EngineError>;

/// Engine error type
#[derive(Debug)]
pub enum EngineError {
    /// IO error
    Io(std::io::Error),
    /// Serialization error
    Serde(serde_json::Error),
    /// Validation error
    Validation(String),
    /// Execution error
    Execution(String),
    /// Network error
    Network(String),
    /// Configuration error
    Config(String),
    /// Generic error
    Generic(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Io(err) => write!(f, "IO error: {}", err),
            EngineError::Serde(err) => write!(f, "Serialization error: {}", err),
            EngineError::Validation(msg) => write!(f, "Validation error: {}", msg),
            EngineError::Execution(msg) => write!(f, "Execution error: {}", msg),
            EngineError::Network(msg) => write!(f, "Network error: {}", msg),
            EngineError::Config(msg) => write!(f, "Configuration error: {}", msg),
            EngineError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EngineError::Io(err) => Some(err),
            EngineError::Serde(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(err: std::io::Error) -> Self {
        EngineError::Io(err)
    }
}

impl From<serde_json::Error> for EngineError {
    fn from(err: serde_json::Error) -> Self {
        EngineError::Serde(err)
    }
}

impl From<String> for EngineError {
    fn from(msg: String) -> Self {
        EngineError::Generic(msg)
    }
}

impl From<&str> for EngineError {
    fn from(msg: &str) -> Self {
        EngineError::Generic(msg.to_string())
    }
}

impl From<serde_yaml::Error> for EngineError {
    fn from(err: serde_yaml::Error) -> Self {
        EngineError::Generic(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for EngineError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        EngineError::Generic(err.to_string())
    }
}

impl From<anyhow::Error> for EngineError {
    fn from(err: anyhow::Error) -> Self {
        EngineError::Generic(err.to_string())
    }
}

impl From<redis::RedisError> for EngineError {
    fn from(err: redis::RedisError) -> Self {
        EngineError::Io(std::io::Error::other(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_error_display() {
        let err = EngineError::Validation("test validation error".to_string());
        assert_eq!(err.to_string(), "Validation error: test validation error");
    }

    #[test]
    fn test_engine_error_from_string() {
        let err: EngineError = "test error".into();
        assert_eq!(err.to_string(), "Error: test error");
    }

    #[test]
    fn test_engine_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: EngineError = io_err.into();
        assert!(matches!(err, EngineError::Io(_)));
    }
}
