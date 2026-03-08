use std::net::SocketAddr;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use serde_json::json;
use tokio::net::TcpListener;

use crate::config::ServerConfig;
use crate::error::EngineResult;
use crate::runtime::EngineRuntime;

#[derive(Clone)]
struct HealthState {
    runtime: Arc<EngineRuntime>,
}

pub async fn start_health_server(
    config: Arc<ServerConfig>,
    runtime: Arc<EngineRuntime>,
) -> EngineResult<()> {
    let state = HealthState { runtime };
    let addr = SocketAddr::from(([0, 0, 0, 0], config.health_port));
    tracing::info!("Health server listening on {}", addr);

    let app = axum::Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/startupz", get(startupz))
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    tracing::info!("Health server stopped");
    Ok(())
}

async fn healthz() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn readyz(State(state): State<HealthState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "boot_time": state.runtime.boot_time().to_rfc3339()
    }))
}

async fn startupz(State(state): State<HealthState>) -> impl IntoResponse {
    let runtime = state.runtime.clone();
    let redis_ok = runtime.health_check_redis().await;
    let nats_ok = runtime.health_check_nats();
    let db_ok = runtime.health_check_db().await;
    let all_healthy = redis_ok && nats_ok && db_ok;
    let status = if all_healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if all_healthy { "ok" } else { "degraded" },
            "checks": {
                "redis": if redis_ok { "ok" } else { "error" },
                "nats": if nats_ok { "ok" } else { "error" },
                "db": if db_ok { "ok" } else { "error" }
            }
        })),
    )
}
