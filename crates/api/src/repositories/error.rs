use thiserror::Error;

pub type RepositoryResult<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Database query failed: {0}")]
    QueryFailed(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Connection pool error: {0}")]
    PoolError(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => RepositoryError::NotFound("record".to_string()),
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
                RepositoryError::PoolError(err.to_string())
            },
            _ => RepositoryError::Database(err.to_string()),
        }
    }
}
