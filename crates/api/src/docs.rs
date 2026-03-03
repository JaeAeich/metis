use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::logs::{LogLineListResponse, LogLineResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::runs::list_runs,
        crate::api::runs::get_run_log,
        crate::api::runs::get_run_status,
        crate::api::tasks::list_tasks,
        crate::api::tasks::get_task,
        crate::api::logs::list_log_lines,
        crate::api::logs::stream_log_lines,
    ),
    components(
        schemas(
            common::models::RunListResponse,
            common::models::RunLog,
            common::models::RunStatus,
            common::models::TaskListResponse,
            common::models::TaskLog,
            LogLineResponse,
            LogLineListResponse,
        )
    ),
    tags(
        (name = "Runs", description = "Workflow run management endpoints"),
    ),
    info(
        title = "Metis API",
        version = "0.1.0",
        description = "Workflow Execution Service API for managing workflow runs"
    )
)]
pub struct ApiDoc;

pub fn docs() -> Router<()> {
    SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()).into()
}
