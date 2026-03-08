use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;

use crate::api::{
    cancel_run, create_run, delete_run, get_run_log, get_run_status, get_service_info, get_task,
    healthz, list_log_lines, list_runs, list_tasks, readyz, startupz, stream_log_lines,
    stream_run_status,
};
use crate::state::AppState;

pub fn get_router(app_state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/startupz", get(startupz))
        .route("/service-info", get(get_service_info))
        .route("/runs", get(list_runs).post(create_run))
        .route("/runs/{run_id}", get(get_run_log).delete(delete_run))
        .route("/runs/{run_id}/cancel", post(cancel_run))
        .route("/runs/{run_id}/status", get(get_run_status))
        .route("/runs/{run_id}/status/stream", get(stream_run_status))
        .route("/runs/{run_id}/tasks", get(list_tasks))
        .route("/runs/{run_id}/tasks/{task_id}", get(get_task))
        .route("/runs/{run_id}/logs", get(list_log_lines))
        .route("/runs/{run_id}/logs/stream", get(stream_log_lines))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(tower_http::trace::DefaultOnResponse::new().level(Level::INFO))
                .on_failure(tower_http::trace::DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}
