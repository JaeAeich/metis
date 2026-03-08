use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::logs::{LogLineListResponse, LogLineResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::service_info::get_service_info,
        crate::api::runs::list_runs,
        crate::api::runs::create_run,
        crate::api::runs::get_run_log,
        crate::api::runs::get_run_status,
        crate::api::runs::stream_run_status,
        crate::api::runs::cancel_run,
        crate::api::runs::delete_run,
        crate::api::tasks::list_tasks,
        crate::api::tasks::get_task,
        crate::api::logs::list_log_lines,
        crate::api::logs::stream_log_lines,
        crate::api::health::healthz,
        crate::api::health::readyz,
        crate::api::health::startupz,
    ),
    components(
        schemas(
            common::models::ServiceInfo,
            common::models::EngineInfo,
            common::configs::WorkflowParamsConfig,
            common::configs::WorkflowParamsStyle,
            common::configs::WorkflowParamsMethod,
            common::configs::WorkflowParamsFormat,
            common::configs::EngineParamsConfig,
            common::configs::UnknownBehavior,
            common::configs::EngineParam,
            common::configs::ParamType,
            common::configs::BooleanStyle,
            common::configs::Validation,
            common::configs::DeniedParam,
            common::configs::MatchType,
            common::configs::Backend,
            common::models::WorkflowTypeVersion,
            common::models::WorkflowEngineVersion,
            common::models::DefaultWorkflowEngineParameter,
            common::models::RunListResponse,
            common::models::RunLog,
            common::models::RunRequest,
            common::models::RunId,
            common::models::RunStatus,
            common::models::TaskListResponse,
            common::models::TaskLog,
            LogLineResponse,
            LogLineListResponse,
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Service Info", description = "Service information endpoints"),
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
