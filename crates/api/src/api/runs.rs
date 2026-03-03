use axum::Json;
use axum::extract::{Path, Query, State};
use common::models::{RunListResponse, RunLog, RunStatus};

use crate::api::{ApiError, ApiResult};
use crate::extractors::{PageParams, RunFilterParams};
use crate::repositories::RunId;
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
            workflow_engine: run.workflow_engine,
            workflow_engine_version: run.workflow_engine_version,
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
