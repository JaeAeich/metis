use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::state::AppState;

#[utoipa::path(get, path = "/healthz", tag = "Health",
    responses((status = 200, description = "Service is alive"))
)]
pub async fn healthz() -> impl IntoResponse {
    (
        StatusCode::OK,
        serde_json::to_string(&json!({ "status": "ok" }))
            .unwrap_or_else(|_| r#"{"status":"error"}"#.to_string()),
    )
}

#[utoipa::path(get, path = "/readyz", tag = "Health",
    responses((status = 200, description = "Service is ready"))
)]
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let boot_time = state.boot_time.read().await;
    let boot_time_str = boot_time
        .as_ref()
        .map(|t| t.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string());

    (
        StatusCode::OK,
        serde_json::to_string(&json!({
            "status": "ok",
            "boot_time": boot_time_str
        }))
        .unwrap(),
    )
}

#[utoipa::path(get, path = "/startupz", tag = "Health",
    responses(
        (status = 200, description = "All dependencies healthy"),
        (status = 503, description = "One or more dependencies degraded"),
    )
)]
pub async fn startupz(State(state): State<AppState>) -> impl IntoResponse {
    let redis_ok = state.services.runs.redis().health_check().await;
    let nats_ok = state.services.runs.nats().health_check().await;
    let db_ok = state.db.health_check().await;

    let all_healthy = redis_ok && nats_ok && db_ok;
    let status = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        serde_json::to_string(&json!({
            "status": if all_healthy { "ok" } else { "degraded" },
            "checks": {
                "redis": if redis_ok { "ok" } else { "error" },
                "nats": if nats_ok { "ok" } else { "error" },
                "db": if db_ok { "ok" } else { "error" }
            }
        }))
        .unwrap(),
    )
}
