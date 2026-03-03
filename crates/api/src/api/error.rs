use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::models::ErrorResponse;
use utoipa::ToSchema;

use crate::services::ServiceError;

pub type ApiResult<T> = std::result::Result<T, ApiError>;

#[derive(Debug, ToSchema)]
pub enum ApiError {
    #[schema(example = "Bad request: invalid parameter")]
    BadRequest(String),

    #[schema(example = "Unauthorized")]
    Unauthorized,

    #[schema(example = "Forbidden")]
    Forbidden,

    #[schema(example = "Run not found: abc123")]
    NotFound(String),

    #[schema(example = "Internal server error")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            },
        };

        let body = Json(ErrorResponse {
            msg: Some(msg),
            status_code: Some(status.as_u16() as i32),
        });

        (status, body).into_response()
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::RunNotFound(id) => ApiError::NotFound(format!("Run not found: {}", id)),
            ServiceError::TaskNotFound(id) => ApiError::NotFound(format!("Task not found: {}", id)),
            ServiceError::InvalidFilter(msg) => ApiError::BadRequest(msg),
            ServiceError::Repository(e) => ApiError::Internal(e.to_string()),
        }
    }
}
