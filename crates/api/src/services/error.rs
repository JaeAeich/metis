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

    #[error(transparent)]
    Repository(#[from] RepositoryError),
}
