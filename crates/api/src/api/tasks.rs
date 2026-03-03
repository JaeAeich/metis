use axum::Json;
use axum::extract::{Path, Query, State};
use common::models::{TaskListResponse, TaskLog};

use crate::api::{ApiError, ApiResult};
use crate::extractors::PageParams;
use crate::repositories::{RunId, TaskId};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/runs/{run_id}/tasks",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID"),
        PageParams
    ),
    responses(
        (status = 200, description = "List of tasks", body = TaskListResponse),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_tasks(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
    Query(page): Query<PageParams>,
) -> ApiResult<Json<TaskListResponse>> {
    let id = RunId::new(run_id);
    let pagination = page.into_pagination();

    let result = app.services.tasks.find_by_run(&id, pagination).await?;

    let task_logs: Vec<TaskLog> = result
        .items
        .into_iter()
        .map(|t| TaskLog {
            id: t.id.into_inner(),
            name: t.name,
            cmd: t.cmd,
            start_time: t.start_time.map(|t| t.to_rfc3339()),
            end_time: t.end_time.map(|t| t.to_rfc3339()),
            stdout: t.stdout,
            stderr: t.stderr,
            exit_code: t.exit_code,
            system_logs: t.system_logs,
            tes_uri: t.tes_uri,
        })
        .collect();

    Ok(Json(TaskListResponse {
        task_logs: Some(task_logs),
        next_page_token: result.next_page_token,
    }))
}

#[utoipa::path(
    get,
    path = "/runs/{run_id}/tasks/{task_id}",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID"),
        ("task_id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Task details", body = TaskLog),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_task(
    State(app): State<AppState>,
    Path((run_id, task_id)): Path<(String, String)>,
) -> ApiResult<Json<TaskLog>> {
    let run = RunId::new(run_id);
    let task = TaskId::new(task_id);

    let t = app
        .services
        .tasks
        .find_by_id(&run, &task)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {}", task)))?;

    Ok(Json(TaskLog {
        id: t.id.into_inner(),
        name: t.name,
        cmd: t.cmd,
        start_time: t.start_time.map(|t| t.to_rfc3339()),
        end_time: t.end_time.map(|t| t.to_rfc3339()),
        stdout: t.stdout,
        stderr: t.stderr,
        exit_code: t.exit_code,
        system_logs: t.system_logs,
        tes_uri: t.tes_uri,
    }))
}
