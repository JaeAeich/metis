use thiserror::Error;

use crate::repositories::RepositoryError;

pub type ServiceResult<T> = std::result::Result<T, ServiceError>;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Run not found: {0}")]
    RunNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Invalid state for operation: {0}")]
    InvalidState(String),

    #[error("Messaging error: {0}")]
    Messaging(String),

    #[error("Validation failed: {0}")]
    Validation(common::errors::ValidationError),

    #[error("Engine config not found for {0}/{1}")]
    EngineConfigNotFound(String, String),

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
