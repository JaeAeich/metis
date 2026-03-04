use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api::{
    cancel_run, create_run, get_run_log, get_run_status, get_task, list_log_lines, list_runs,
    list_tasks, stream_log_lines,
};
use crate::state::AppState;

pub fn get_router(app_state: AppState) -> Router {
    Router::new()
        .route("/runs", get(list_runs).post(create_run))
        .route("/runs/{run_id}", get(get_run_log))
        .route("/runs/{run_id}/cancel", post(cancel_run))
        .route("/runs/{run_id}/status", get(get_run_status))
        .route("/runs/{run_id}/tasks", get(list_tasks))
        .route("/runs/{run_id}/tasks/{task_id}", get(get_task))
        .route("/runs/{run_id}/logs", get(list_log_lines))
        .route("/runs/{run_id}/logs/stream", get(stream_log_lines))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state)
}
