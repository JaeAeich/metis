use axum::Json;
use axum::extract::State;
use common::models::{ServiceInfo, Stats};

use crate::api::{ApiError, ApiResult};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/service-info",
    tag = "Service Info",
    responses(
        (status = 200, description = "Service information (WES spec compliant)", body = ServiceInfo),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_service_info(State(app): State<AppState>) -> ApiResult<Json<ServiceInfo>> {
    let info = app
        .services
        .service_info
        .get_service_info()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(info))
}

#[utoipa::path(
    get,
    path = "/stats",
    tag = "Stats",
    responses(
        (status = 200, description = "System and engine statistics", body = Stats),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_stats(State(app): State<AppState>) -> ApiResult<Json<Stats>> {
    let stats = app
        .services
        .service_info
        .get_stats()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(stats))
}
