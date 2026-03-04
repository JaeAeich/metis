use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;

use crate::api::ApiError;

pub struct Json<T>(pub T);

impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Json(value)),
            Err(rejection) => {
                let msg = match rejection {
                    JsonRejection::JsonDataError(e) => e.to_string(),
                    JsonRejection::JsonSyntaxError(e) => e.to_string(),
                    JsonRejection::MissingJsonContentType(e) => e.to_string(),
                    e => e.to_string(),
                };
                Err(ApiError::BadRequest(msg))
            },
        }
    }
}
