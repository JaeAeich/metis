use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Parameter '{0}' is denied: {1}")]
    DeniedParameter(String, String),

    #[error("Unknown parameter '{0}' and unknownEngineParamsBehavior is 'reject'")]
    UnknownParameter(String),

    #[error("Validation failed for '{0}': {1}")]
    ValidationFailed(String, String),

    #[error("Required parameter '{0}' is missing")]
    RequiredMissing(String),

    #[error("Type mismatch for '{0}': expected {1}")]
    TypeMismatch(String, String),

    #[error("JSON parse error for '{0}': {1}")]
    JsonParseError(String, String),
}
