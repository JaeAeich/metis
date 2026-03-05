use axum::Json;
use axum::extract::{Path, Query, State};
use common::models::{RunId as RunIdModel, RunListResponse, RunLog, RunRequest, RunStatus};
use uuid::Uuid;

use crate::api::{ApiError, ApiResult};
use crate::extractors::{Json as ValidatedJson, PageParams, RunFilterParams};
use crate::repositories::RunId;
use crate::services::ServiceError;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/runs",
    tag = "Runs",
    params(PageParams, RunFilterParams),
    responses(
        (status = 200, description = "List of workflow runs", body = RunListResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_runs(
    State(app): State<AppState>,
    Query(page): Query<PageParams>,
    Query(filter): Query<RunFilterParams>,
) -> ApiResult<Json<RunListResponse>> {
    let filter = filter.into_filter().map_err(ApiError::BadRequest)?;
    let pagination = page.into_pagination();

    let result = app.services.runs.find_all(filter, pagination).await?;

    let runs: Vec<common::models::RunListResponseRunsInner> = result
        .items
        .into_iter()
        .map(|r| common::models::RunListResponseRunsInner {
            run_id: r.id.into_inner(),
            state: Some(r.state),
            start_time: r.start_time.map(|t| t.to_rfc3339()),
            end_time: r.end_time.map(|t| t.to_rfc3339()),
            tags: serde_json::from_value(r.tags).unwrap_or_default(),
        })
        .collect();

    Ok(Json(RunListResponse {
        runs: Some(runs),
        next_page_token: result.next_page_token,
    }))
}

#[utoipa::path(
    get,
    path = "/runs/{run_id}",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID")
    ),
    responses(
        (status = 200, description = "Run log details", body = RunLog),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_run_log(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<RunLog>> {
    let id = RunId::new(run_id);

    let run = app
        .services
        .runs
        .find_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run not found: {}", id)))?;

    let task_logs_url = format!("/runs/{}/tasks", run.id);

    Ok(Json(RunLog {
        run_id: Some(run.id.into_inner()),
        request: Some(Box::new(common::models::RunRequest {
            workflow_params: run.workflow_params,
            workflow_type: run.workflow_type,
            workflow_type_version: run.workflow_type_version,
            tags: serde_json::from_value(run.tags).ok(),
            workflow_engine_parameters: serde_json::from_value(
                run.workflow_engine_parameters.unwrap_or(serde_json::Value::Null),
            )
            .ok(),
            workflow_engine: run.workflow_engine.unwrap_or_default(),
            workflow_engine_version: run.workflow_engine_version.unwrap_or_default(),
            workflow_url: run.workflow_url,
        })),
        state: Some(run.state),
        run_log: None,
        task_logs_url: Some(task_logs_url),
        outputs: None,
    }))
}

#[utoipa::path(
    get,
    path = "/runs/{run_id}/status",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID")
    ),
    responses(
        (status = 200, description = "Run status", body = RunStatus),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_run_status(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<RunStatus>> {
    let id = RunId::new(run_id);

    let run = app
        .services
        .runs
        .find_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run not found: {}", id)))?;

    Ok(Json(RunStatus {
        run_id: run.id.into_inner(),
        state: Some(run.state),
    }))
}

#[utoipa::path(
    post,
    path = "/runs",
    tag = "Runs",
    request_body = RunRequest,
    responses(
        (status = 200, description = "Run created and queued", body = RunIdModel),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_run(
    State(app): State<AppState>,
    ValidatedJson(req): ValidatedJson<RunRequest>,
) -> ApiResult<Json<RunIdModel>> {
    let run_id = Uuid::now_v7().to_string();
    let user_id = "default";

    app.services
        .runs
        .create_run(&run_id, user_id, &req)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(RunIdModel { run_id: Some(run_id) }))
}

#[utoipa::path(
    post,
    path = "/runs/{run_id}/cancel",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID")
    ),
    responses(
        (status = 200, description = "Cancel request accepted"),
        (status = 400, description = "Run cannot be canceled in its current state"),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cancel_run(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let id = RunId::new(run_id);

    app.services.runs.request_cancel(&id).await.map_err(|e| match e {
        ServiceError::RunNotFound(msg) => ApiError::NotFound(msg),
        ServiceError::InvalidState(msg) => ApiError::BadRequest(msg),
        other => ApiError::Internal(other.to_string()),
    })?;

    Ok(Json(serde_json::json!({ "status": "cancel requested" })))
}

#[utoipa::path(
    delete,
    path = "/runs/{run_id}",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID")
    ),
    responses(
        (status = 200, description = "Run deleted", body = RunIdModel),
        (status = 400, description = "Run cannot be deleted in current state"),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_run(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
) -> ApiResult<Json<RunIdModel>> {
    let id = RunId::new(run_id);

    app.services.runs.delete_run(&id).await?;

    Ok(Json(RunIdModel { run_id: Some(id.into_inner()) }))
}
