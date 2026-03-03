use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::configs::EngineParam;

/// Available workflow types supported by a given instance of the service.
///
/// # Example
/// ```json
/// {
///   "workflow_type_version": ["1.0", "1.1", "1.2"]
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct WorkflowTypeVersion {
    /// An array of one or more acceptable types for the `workflow_type`
    ///
    /// # Examples
    /// - `["1.0", "1.1"]` for CWL versions
    /// - `["draft-2", "1.0"]` for WDL versions
    #[serde(rename = "workflow_type_version", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["1.0", "1.1", "1.2"]))]
    pub workflow_type_version: Option<Vec<String>>,
}

/// A message that allows one to describe default parameters for a workflow
/// engine.
///
/// # Example
/// ```json
/// {
///   "name": "memory",
///   "type": "string",
///   "default_value": "4GB"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct DefaultWorkflowEngineParameter {
    /// The name of the parameter
    ///
    /// # Examples
    /// - `"memory"` for memory allocation
    /// - `"cpu"` for CPU allocation
    /// - `"disk"` for disk space
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    #[schema(example = "memory")]
    pub name: Option<String>,

    /// Describes the type of the parameter, e.g. float.
    ///
    /// # Examples
    /// - `"string"` for text values
    /// - `"float"` for decimal numbers
    /// - `"integer"` for whole numbers
    /// - `"boolean"` for true/false values
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[schema(example = "string")]
    pub r#type: Option<String>,

    /// The stringified version of the default parameter.
    ///
    /// # Examples
    /// - `"2.45"` for a float parameter
    /// - `"4GB"` for a memory parameter
    /// - `"true"` for a boolean parameter
    #[serde(rename = "default_value", skip_serializing_if = "Option::is_none")]
    #[schema(example = "4GB")]
    pub default_value: Option<String>,
}

/// Service information including supported versions, protocols, and system
/// statistics.
///
/// # Example
/// ```json
/// {
///   "workflow_type_versions": {
///     "CWL": {"workflow_type_version": ["1.0", "1.1"]},
///     "WDL": {"workflow_type_version": ["1.0"]}
///   },
///   "supported_wes_versions": ["1.0.0"],
///   "supported_filesystem_protocols": ["http", "https", "s3", "file"],
///   "system_state_counts": {
///     "COMPLETE": 150,
///     "RUNNING": 5,
///     "QUEUED": 10
///   },
///   "auth_instructions_url": "https://example.com/auth",
///   "tags": {
///     "environment": "production",
///     "version": "1.0.0"
///   }
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ServiceInfo {
    /// Map of workflow types to their supported versions
    #[serde(rename = "workflow_type_versions")]
    #[schema(example = json!({"CWL": {"workflow_type_version": ["1.0", "1.1"]}, "WDL": {"workflow_type_version": ["1.0"]}}))]
    pub workflow_type_versions: std::collections::HashMap<String, WorkflowTypeVersion>,

    /// The version(s) of the WES schema supported by this service
    #[serde(rename = "supported_wes_versions")]
    #[schema(example = json!(["1.0.0", "1.1.0"]))]
    pub supported_wes_versions: Vec<String>,

    /// The filesystem protocols supported by this service.
    ///
    /// Currently these may include common protocols using the terms 'http',
    /// 'https', 'sftp', 's3', 'gs', 'file', or 'synapse', but others are
    /// possible and the terms beyond these core protocols are currently not
    /// fixed. This section reports those protocols (either common or not)
    /// supported by this WES service.
    #[serde(rename = "supported_filesystem_protocols")]
    #[schema(example = json!(["http", "https", "s3", "file", "gs"]))]
    pub supported_filesystem_protocols: Vec<String>,

    /// Map of workflow engines to their supported versions
    #[serde(rename = "workflow_engine_versions")]
    #[schema(example = json!({"cromwell": {"workflow_engine_version": ["85", "86"]}, "nextflow": {"workflow_engine_version": ["23.04.0"]}}))]
    pub workflow_engine_versions: std::collections::HashMap<String, WorkflowEngineVersion>,

    /// Each workflow engine can present additional parameters that can be sent
    /// to the workflow engine. This message will list the default values,
    /// and their types for each workflow engine.
    #[serde(rename = "default_workflow_engine_parameters")]
    pub default_workflow_engine_parameters: Vec<DefaultWorkflowEngineParameter>,

    /// The system statistics, key is the statistic, value is the count of runs
    /// in that state. See the State enum for the possible keys.
    #[serde(rename = "system_state_counts")]
    #[schema(example = json!({"COMPLETE": 150, "RUNNING": 5, "QUEUED": 10, "UNKNOWN": 0}))]
    pub system_state_counts: std::collections::HashMap<String, i64>,

    /// A web page URL with human-readable instructions on how to get an
    /// authorization token for use with a specific WES endpoint.
    #[serde(rename = "auth_instructions_url")]
    #[schema(example = "https://example.com/auth")]
    pub auth_instructions_url: String,

    /// Arbitrary key-value pairs for service metadata
    #[serde(rename = "tags")]
    #[schema(example = json!({"environment": "production", "version": "1.0.0"}))]
    pub tags: std::collections::HashMap<String, String>,
}

/// An object that can optionally include information about the error.
///
/// # Example
/// ```json
/// {
///   "msg": "Workflow not found",
///   "status_code": 404
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct ErrorResponse {
    /// A detailed error message.
    #[serde(rename = "msg", skip_serializing_if = "Option::is_none")]
    #[schema(example = "Workflow not found")]
    pub msg: Option<String>,

    /// The integer representing the HTTP status code (e.g. 200, 404).
    #[serde(rename = "status_code", skip_serializing_if = "Option::is_none")]
    #[schema(example = 404)]
    pub status_code: Option<i32>,
}

/// Log and other info for workflow execution.
///
/// # Example
/// ```json
/// {
///   "name": "main_workflow",
///   "cmd": ["cwltool", "--outdir", "/tmp/output", "workflow.cwl", "inputs.json"],
///   "start_time": "2024-01-15T10:30:00Z",
///   "end_time": "2024-01-15T11:45:30Z",
///   "stdout": "https://storage.example.com/logs/stdout.log",
///   "stderr": "https://storage.example.com/logs/stderr.log",
///   "exit_code": 0,
///   "system_logs": ["Started execution on node-01", "Memory usage: 2.1GB"]
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct Log {
    /// The task or workflow name
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    #[schema(example = "main_workflow")]
    pub name: Option<String>,

    /// The command line that was executed
    #[serde(rename = "cmd", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["cwltool", "--outdir", "/tmp/output", "workflow.cwl", "inputs.json"]))]
    #[sqlx(json)]
    pub cmd: Option<Vec<String>>,

    /// When the command started executing, in ISO 8601 format
    /// "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub start_time: Option<String>,

    /// When the command stopped executing (completed, failed, or cancelled), in
    /// ISO 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T11:45:30Z")]
    pub end_time: Option<String>,

    /// A URL to retrieve standard output logs of the workflow run or task.
    /// This URL may change between status requests, or may not be available
    /// until the task or workflow has finished execution.
    /// Should be available using the same credentials used to access the WES
    /// endpoint.
    #[serde(rename = "stdout", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://storage.example.com/logs/stdout.log")]
    pub stdout: Option<String>,

    /// A URL to retrieve standard error logs of the workflow run or task.
    /// This URL may change between status requests, or may not be available
    /// until the task or workflow has finished execution.
    /// Should be available using the same credentials used to access the WES
    /// endpoint.
    #[serde(rename = "stderr", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://storage.example.com/logs/stderr.log")]
    pub stderr: Option<String>,

    /// Exit code of the program
    #[serde(rename = "exit_code", skip_serializing_if = "Option::is_none")]
    #[schema(example = 0)]
    pub exit_code: Option<i32>,

    /// System logs are any logs the system decides are relevant, which are not
    /// tied directly to a workflow. Content is implementation specific:
    /// format, size, etc. System logs may be collected here to provide
    /// convenient access. For example, the system may include an error
    /// message that caused a SYSTEM_ERROR state (e.g. disk is full), etc.
    #[serde(rename = "system_logs", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["Started execution on node-01", "Memory usage: 2.1GB"]))]
    #[sqlx(json)]
    pub system_logs: Option<Vec<String>>,
}

/// Simple wrapper for workflow run ID.
///
/// # Example
/// ```json
/// {
///   "run_id": "550e8400-e29b-41d4-a716-446655440000"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct RunId {
    /// Workflow run ID (typically a UUID)
    #[serde(rename = "run_id", skip_serializing_if = "Option::is_none")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub run_id: Option<String>,
}

/// The service will return a RunListResponse when receiving a successful
/// RunListRequest.
///
/// **DEPRECATION WARNING**: The use of `RunStatus` as the schema for `runs`
/// array items will not be permitted from the next major version of the
/// specification (2.0.0) onwards. We encourage implementers to use `RunSummary`
/// instead.
///
/// # Example
/// ```json
/// {
///   "runs": [
///     {
///       "run_id": "550e8400-e29b-41d4-a716-446655440000",
///       "state": "COMPLETE",
///       "start_time": "2024-01-15T10:30:00Z",
///       "end_time": "2024-01-15T11:45:30Z",
///       "tags": {"project": "genomics", "priority": "high"}
///     }
///   ],
///   "next_page_token": "eyJwYWdlIjoyLCJsaW1pdCI6NTB9"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunListResponse {
    /// A list of workflow runs that the service has executed or is executing.
    /// The list is filtered to only include runs that the caller has permission
    /// to see.
    #[serde(rename = "runs", skip_serializing_if = "Option::is_none")]
    pub runs: Option<Vec<RunListResponseRunsInner>>,

    /// A token which may be supplied as `page_token` in workflow run list
    /// request to get the next page of results. An empty string indicates
    /// there are no more items to return.
    #[serde(rename = "next_page_token", skip_serializing_if = "Option::is_none")]
    #[schema(example = "eyJwYWdlIjoyLCJsaW1pdCI6NTB9")]
    pub next_page_token: Option<String>,
}

/// Individual run entry in the run list response.
///
/// # Example
/// ```json
/// {
///   "run_id": "550e8400-e29b-41d4-a716-446655440000",
///   "state": "COMPLETE",
///   "start_time": "2024-01-15T10:30:00Z",
///   "end_time": "2024-01-15T11:45:30Z",
///   "tags": {"project": "genomics", "priority": "high"}
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct RunListResponseRunsInner {
    /// Unique identifier for the workflow run
    #[serde(rename = "run_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub run_id: String,

    /// Current state of the workflow run
    #[serde(rename = "state", skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,

    /// When the run started executing, in ISO 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub start_time: Option<String>,

    /// When the run stopped executing (completed, failed, or cancelled), in ISO
    /// 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T11:45:30Z")]
    pub end_time: Option<String>,

    /// Arbitrary key/value tags added by the client during run creation
    #[serde(rename = "tags")]
    #[schema(example = json!({"project": "genomics", "priority": "high"}))]
    #[sqlx(json)]
    pub tags: std::collections::HashMap<String, String>,
}

/// Complete workflow run log including request details, state, and execution
/// logs.
///
/// # Example
/// ```json
/// {
///   "run_id": "550e8400-e29b-41d4-a716-446655440000",
///   "request": {
///     "workflow_params": {"input_file": "s3://bucket/input.vcf"},
///     "workflow_type": "CWL",
///     "workflow_type_version": "1.0",
///     "workflow_url": "https://github.com/example/workflow.cwl"
///   },
///   "state": "COMPLETE",
///   "run_log": {
///     "name": "variant_calling",
///     "start_time": "2024-01-15T10:30:00Z",
///     "end_time": "2024-01-15T11:45:30Z",
///     "exit_code": 0
///   },
///   "task_logs_url": "https://api.example.com/runs/550e8400-e29b-41d4-a716-446655440000/tasks",
///   "outputs": {"result_file": "s3://bucket/output.vcf"}
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunLog {
    /// Workflow run ID
    #[serde(rename = "run_id", skip_serializing_if = "Option::is_none")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub run_id: Option<String>,

    /// The original request that started this workflow run
    #[serde(rename = "request", skip_serializing_if = "Option::is_none")]
    pub request: Option<Box<RunRequest>>,

    /// Current state of the workflow run
    #[serde(rename = "state", skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,

    /// Execution log for the workflow run
    #[serde(rename = "run_log", skip_serializing_if = "Option::is_none")]
    pub run_log: Option<Box<Log>>,

    /// A reference to the complete url which may be used to obtain a paginated
    /// list of task logs for this workflow
    #[serde(rename = "task_logs_url", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://api.example.com/runs/550e8400-e29b-41d4-a716-446655440000/tasks")]
    pub task_logs_url: Option<String>,

    /// The outputs from the workflow run, based on engine the top level
    /// categories might differ
    #[serde(rename = "outputs", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!({
        "results": {
            "output.vcf": {
                "url": "s3://bucket/run-123/results/output.vcf",
                "name": "output.vcf",
                "size": 18234,
                "checksum": "sha1$abc123",
                "created_at": "2025-10-18T09:12:00Z"
            },
            "summary.txt": {
                "url": "s3://bucket/run-123/results/summary.txt",
                "name": "summary.txt"
            }
        },
        "logs": {
            "workflow.log": {
                "url": "s3://bucket/run-123/logs/workflow.log",
                "name": "workflow.log",
                "size": 3456
            }
        }
    }))]
    pub outputs: Option<
        std::collections::HashMap<String, std::collections::HashMap<String, WorkflowFileInfo>>,
    >,
}

/// To execute a workflow, send a run request including all the details needed
/// to begin downloading and executing a given workflow. If workflow_engine and
/// workflow_engine_version are not provided, servers can use the most recent
/// workflow_engine_version of workflow_engine that WES instance uses to process
/// the request if supports for the requested workflow_type.
///
/// # Example
/// ```json
/// {
///   "workflow_params": {
///     "input_file": "s3://my-bucket/input.vcf",
///     "reference_genome": "hg38",
///     "output_prefix": "sample123"
///   },
///   "workflow_type": "CWL",
///   "workflow_type_version": "1.0",
///   "tags": {
///     "project": "genomics",
///     "sample_id": "sample123",
///     "priority": "high"
///   },
///   "workflow_engine_parameters": {
///     "memory": "8GB",
///     "cpu": "4"
///   },
///   "workflow_engine": "cwltool",
///   "workflow_engine_version": "3.1.20240112164112",
///   "workflow_url": "https://github.com/example/variant-calling.cwl"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct RunRequest {
    /// **REQUIRED** The workflow run parameterizations (JSON encoded),
    /// including input and output file locations
    #[serde(rename = "workflow_params", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!({"input_file": "s3://my-bucket/input.vcf", "reference_genome": "hg38"}))]
    #[sqlx(json)]
    pub workflow_params: Option<serde_json::Value>,

    /// **REQUIRED** The workflow descriptor type, must be "CWL" or "WDL"
    /// currently (or another alternative supported by this WES instance)
    #[serde(rename = "workflow_type")]
    #[schema(example = "CWL")]
    pub workflow_type: String,

    /// **REQUIRED** The workflow descriptor type version, must be one supported
    /// by this WES instance
    #[serde(rename = "workflow_type_version")]
    #[schema(example = "1.0")]
    pub workflow_type_version: String,

    /// Arbitrary key/value tags for organizing and filtering workflow runs
    #[serde(rename = "tags", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!({"project": "genomics", "sample_id": "sample123", "priority": "high"}))]
    #[sqlx(json)]
    pub tags: Option<std::collections::HashMap<String, String>>,

    /// Additional parameters to pass to the workflow engine
    #[serde(rename = "workflow_engine_parameters", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!({"memory": "8GB", "cpu": "4"}))]
    #[sqlx(json)]
    pub workflow_engine_parameters: Option<std::collections::HashMap<String, String>>,

    /// The workflow engine, must be one supported by this WES instance.
    /// Required if workflow_engine_version is provided.
    #[serde(rename = "workflow_engine", skip_serializing_if = "Option::is_none")]
    #[schema(example = "cwltool")]
    pub workflow_engine: Option<String>,

    /// The workflow engine version, must be one supported by this WES instance.
    /// If workflow_engine is provided, but workflow_engine_version is not,
    /// servers can make no assumptions with regard to the engine version
    /// the WES instance uses to process the request if that WES instance
    /// supports multiple versions of the requested engine.
    #[serde(rename = "workflow_engine_version", skip_serializing_if = "Option::is_none")]
    #[schema(example = "3.1.20240112164112")]
    pub workflow_engine_version: Option<String>,

    /// **REQUIRED** The workflow CWL or WDL document. When
    /// `workflow_attachments` is used to attach files, the `workflow_url`
    /// may be a relative path to one of the attachments.
    #[serde(rename = "workflow_url")]
    #[schema(example = "https://github.com/example/variant-calling.cwl")]
    pub workflow_url: String,
}

/// Small description of a workflow run with essential information.
///
/// This is the recommended replacement for the deprecated `RunStatus` model.
///
/// # Example
/// ```json
/// {
///   "run_id": "550e8400-e29b-41d4-a716-446655440000",
///   "state": "COMPLETE",
///   "start_time": "2024-01-15T10:30:00Z",
///   "end_time": "2024-01-15T11:45:30Z",
///   "tags": {
///     "project": "genomics",
///     "sample_id": "sample123",
///     "user": "scientist@example.com"
///   }
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct RunSummary {
    /// Unique identifier for the workflow run
    #[serde(rename = "run_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub run_id: String,

    /// Current execution state of the workflow run
    #[serde(rename = "state", skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,

    /// When the run started executing, in ISO 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub start_time: Option<String>,

    /// When the run stopped executing (completed, failed, or cancelled), in ISO
    /// 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T11:45:30Z")]
    pub end_time: Option<String>,

    /// Arbitrary key/value tags added by the client during run creation
    #[serde(rename = "tags")]
    #[schema(example = json!({"project": "genomics", "sample_id": "sample123", "user": "scientist@example.com"}))]
    #[sqlx(json)]
    pub tags: std::collections::HashMap<String, String>,
}

/// State can take any of the following values:
///
/// - `UNKNOWN`: The state of the task is unknown. This provides a safe default
///   for messages where this field is missing.
/// - `QUEUED`: The task is queued.
/// - `INITIALIZING`: The task has been assigned to a worker and is currently
///   preparing to run.
/// - `RUNNING`: The task is running. Input files are downloaded and the first
///   Executor has been started.
/// - `PAUSED`: The task is paused. An implementation may have the ability to
///   pause a task, but this is not required.
/// - `COMPLETE`: The task has completed running. Executors have exited without
///   error and output files have been successfully uploaded.
/// - `EXECUTOR_ERROR`: The task encountered an error in one of the Executor
///   processes.
/// - `SYSTEM_ERROR`: The task was stopped due to a system error, but not from
///   an Executor.
/// - `CANCELED`: The task was canceled by the user.
/// - `CANCELING`: The task was canceled by the user, and is in the process of
///   stopping.
/// - `PREEMPTED`: The task is stopped (preempted) by the system.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    /// The state of the task is unknown. Safe default for missing fields.
    #[serde(rename = "UNKNOWN")]
    #[default]
    Unknown,
    /// The task is queued and waiting for resources.
    #[serde(rename = "QUEUED")]
    Queued,
    /// The task has been assigned to a worker and is preparing to run.
    #[serde(rename = "INITIALIZING")]
    Initializing,
    /// The task is currently running.
    #[serde(rename = "RUNNING")]
    Running,
    /// The task is paused (optional capability).
    #[serde(rename = "PAUSED")]
    Paused,
    /// The task has completed successfully.
    #[serde(rename = "COMPLETE")]
    Complete,
    /// The task encountered an error in one of the Executor processes.
    #[serde(rename = "EXECUTOR_ERROR")]
    ExecutorError,
    /// The task was stopped due to a system error.
    #[serde(rename = "SYSTEM_ERROR")]
    SystemError,
    /// The task was canceled by the user.
    #[serde(rename = "CANCELED")]
    Canceled,
    /// The task is in the process of being canceled.
    #[serde(rename = "CANCELING")]
    Canceling,
    /// The task was preempted by the system.
    #[serde(rename = "PREEMPTED")]
    Preempted,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Unknown => write!(f, "UNKNOWN"),
            State::Queued => write!(f, "QUEUED"),
            State::Initializing => write!(f, "INITIALIZING"),
            State::Running => write!(f, "RUNNING"),
            State::Paused => write!(f, "PAUSED"),
            State::Complete => write!(f, "COMPLETE"),
            State::ExecutorError => write!(f, "EXECUTOR_ERROR"),
            State::SystemError => write!(f, "SYSTEM_ERROR"),
            State::Canceled => write!(f, "CANCELED"),
            State::Canceling => write!(f, "CANCELING"),
            State::Preempted => write!(f, "PREEMPTED"),
        }
    }
}

impl std::str::FromStr for State {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNKNOWN" => Ok(State::Unknown),
            "QUEUED" => Ok(State::Queued),
            "INITIALIZING" => Ok(State::Initializing),
            "RUNNING" => Ok(State::Running),
            "PAUSED" => Ok(State::Paused),
            "COMPLETE" => Ok(State::Complete),
            "EXECUTOR_ERROR" => Ok(State::ExecutorError),
            "SYSTEM_ERROR" => Ok(State::SystemError),
            "CANCELED" => Ok(State::Canceled),
            "CANCELING" => Ok(State::Canceling),
            "PREEMPTED" => Ok(State::Preempted),
            _ => Err(format!("Invalid state: {}", s)),
        }
    }
}

// Implement sqlx::Type for State to enable database storage
impl sqlx::Type<sqlx::Postgres> for State {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("TEXT")
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for State {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let state_str = match self {
            State::Unknown => "UNKNOWN",
            State::Queued => "QUEUED",
            State::Initializing => "INITIALIZING",
            State::Running => "RUNNING",
            State::Paused => "PAUSED",
            State::Complete => "COMPLETE",
            State::ExecutorError => "EXECUTOR_ERROR",
            State::SystemError => "SYSTEM_ERROR",
            State::Canceled => "CANCELED",
            State::Canceling => "CANCELING",
            State::Preempted => "PREEMPTED",
        };
        state_str.encode_by_ref(buf)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for State {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let state_str: String = sqlx::Decode::decode(value)?;
        match state_str.as_str() {
            "UNKNOWN" => Ok(State::Unknown),
            "QUEUED" => Ok(State::Queued),
            "INITIALIZING" => Ok(State::Initializing),
            "RUNNING" => Ok(State::Running),
            "PAUSED" => Ok(State::Paused),
            "COMPLETE" => Ok(State::Complete),
            "EXECUTOR_ERROR" => Ok(State::ExecutorError),
            "SYSTEM_ERROR" => Ok(State::SystemError),
            "CANCELED" => Ok(State::Canceled),
            "CANCELING" => Ok(State::Canceling),
            "PREEMPTED" => Ok(State::Preempted),
            _ => Ok(State::Unknown),
        }
    }
}

/// State information of a workflow run.
///
/// # Example
/// ```json
/// {
///   "run_id": "550e8400-e29b-41d4-a716-446655440000",
///   "state": "RUNNING"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunStatus {
    /// Unique identifier for the workflow run
    #[serde(rename = "run_id")]
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub run_id: String,

    /// Current state of the workflow run
    #[serde(rename = "state", skip_serializing_if = "Option::is_none")]
    pub state: Option<State>,
}

/// The service will return a TaskListResponse when receiving a successful
/// TaskListRequest.
///
/// # Example
/// ```json
/// {
///   "task_logs": [
///     {
///       "name": "variant_calling_step_1",
///       "id": "task_550e8400-e29b-41d4-a716-446655440001",
///       "cmd": ["bwa", "mem", "-t", "4", "reference.fa", "input.fastq"],
///       "start_time": "2024-01-15T10:30:00Z",
///       "end_time": "2024-01-15T10:45:30Z",
///       "stdout": "https://storage.example.com/logs/task1_stdout.log",
///       "stderr": "https://storage.example.com/logs/task1_stderr.log",
///       "exit_code": 0
///     }
///   ],
///   "next_page_token": "eyJ0YXNrX29mZnNldCI6NTAsInJ1bl9pZCI6IjU1MGU4NDAwIn0="
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TaskListResponse {
    /// The logs, and other key info like timing and exit code, for each step in
    /// the workflow run.
    #[serde(rename = "task_logs", skip_serializing_if = "Option::is_none")]
    pub task_logs: Option<Vec<TaskLog>>,

    /// A token which may be supplied as `page_token` in workflow run task list
    /// request to get the next page of results. An empty string indicates
    /// there are no more items to return.
    #[serde(rename = "next_page_token", skip_serializing_if = "Option::is_none")]
    #[schema(example = "eyJ0YXNrX29mZnNldCI6NTAsInJ1bl9pZCI6IjU1MGU4NDAwIn0=")]
    pub next_page_token: Option<String>,
}

/// Runtime information for a given task within a workflow run.
///
/// # Example
/// ```json
/// {
///   "name": "variant_calling_step_1",
///   "id": "task_550e8400-e29b-41d4-a716-446655440001",
///   "cmd": ["bwa", "mem", "-t", "4", "reference.fa", "input.fastq"],
///   "start_time": "2024-01-15T10:30:00Z",
///   "end_time": "2024-01-15T10:45:30Z",
///   "stdout": "https://storage.example.com/logs/task1_stdout.log",
///   "stderr": "https://storage.example.com/logs/task1_stderr.log",
///   "exit_code": 0,
///   "system_logs": [
///     "Task assigned to compute node worker-03",
///     "Docker image pulled: biocontainers/bwa:0.7.17",
///     "Task completed successfully"
///   ],
///   "tes_uri": "https://tes.example.com/v1/tasks/task_550e8400-e29b-41d4-a716-446655440001"
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct TaskLog {
    /// The task or workflow step name
    #[serde(rename = "name")]
    #[schema(example = "variant_calling_step_1")]
    pub name: String,

    /// A unique identifier which may be used to reference the task
    #[serde(rename = "id")]
    #[schema(example = "task_550e8400-e29b-41d4-a716-446655440001")]
    pub id: String,

    /// The command line that was executed
    #[serde(rename = "cmd", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["bwa", "mem", "-t", "4", "reference.fa", "input.fastq"]))]
    #[sqlx(json)]
    pub cmd: Option<Vec<String>>,

    /// When the command started executing, in ISO 8601 format
    /// "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub start_time: Option<String>,

    /// When the command stopped executing (completed, failed, or cancelled), in
    /// ISO 8601 format "%Y-%m-%dT%H:%M:%SZ"
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-01-15T10:45:30Z")]
    pub end_time: Option<String>,

    /// A URL to retrieve standard output logs of the workflow run or task.
    /// This URL may change between status requests, or may not be available
    /// until the task or workflow has finished execution.
    /// Should be available using the same credentials used to access the WES
    /// endpoint.
    #[serde(rename = "stdout", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://storage.example.com/logs/task1_stdout.log")]
    pub stdout: Option<String>,

    /// A URL to retrieve standard error logs of the workflow run or task.
    /// This URL may change between status requests, or may not be available
    /// until the task or workflow has finished execution.
    /// Should be available using the same credentials used to access the WES
    /// endpoint.
    #[serde(rename = "stderr", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://storage.example.com/logs/task1_stderr.log")]
    pub stderr: Option<String>,

    /// Exit code of the program
    #[serde(rename = "exit_code", skip_serializing_if = "Option::is_none")]
    #[schema(example = 0)]
    pub exit_code: Option<i32>,

    /// System logs are any logs the system decides are relevant, which are not
    /// tied directly to a task. Content is implementation specific: format,
    /// size, etc. System logs may be collected here to provide convenient
    /// access. For example, the system may include the name of the host
    /// where the task is executing, an error message that caused a
    /// SYSTEM_ERROR state (e.g. disk is full), etc.
    #[serde(rename = "system_logs", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["Task assigned to compute node worker-03", "Docker image pulled: biocontainers/bwa:0.7.17", "Task completed successfully"]))]
    #[sqlx(json)]
    pub system_logs: Option<Vec<String>>,

    /// An optional URL pointing to an extended task definition defined by a [TES api](https://github.com/ga4gh/task-execution-schemas)
    #[serde(rename = "tes_uri", skip_serializing_if = "Option::is_none")]
    #[schema(
        example = "https://tes.example.com/v1/tasks/task_550e8400-e29b-41d4-a716-446655440001"
    )]
    pub tes_uri: Option<String>,
}

/// Available workflow engine versions supported by a given instance of the
/// service.
///
/// # Example
/// ```json
/// {
///   "workflow_engine_version": ["85", "86", "87"]
/// }
/// ```
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
pub struct WorkflowEngineVersion {
    /// An array of one or more acceptable engine versions for the
    /// `workflow_engine`
    ///
    /// # Examples
    /// - `["85", "86"]` for Cromwell versions
    /// - `["23.04.0", "23.10.0"]` for Nextflow versions
    /// - `["3.1.20240112164112"]` for cwltool versions
    #[serde(rename = "workflow_engine_version", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["85", "86", "87"]))]
    pub workflow_engine_version: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ValidatedParam {
    pub spec: EngineParam,
    pub value: Option<serde_json::Value>,
}

/// To execute a workflow, send a run request including all the details needed
/// to begin downloading and executing a given workflow. If workflow_engine and
/// workflow_engine_version are not provided, servers can use the most recent
/// workflow_engine_version of workflow_engine that WES instance uses to process
/// the request if supports for the requested workflow_type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromRow)]
pub struct ValidatedRunRequest {
    /// **REQUIRED** The workflow run parameterizations (JSON encoded),
    /// including input and output file locations
    #[serde(rename = "workflow_params", skip_serializing_if = "Option::is_none")]
    pub workflow_params: Option<serde_json::Value>,

    /// **REQUIRED** The workflow descriptor type, must be "CWL" or "WDL"
    /// currently (or another alternative supported by this WES instance)
    #[serde(rename = "workflow_type")]
    pub workflow_type: String,

    /// **REQUIRED** The workflow descriptor type version, must be one supported
    /// by this WES instance
    #[serde(rename = "workflow_type_version")]
    pub workflow_type_version: String,

    /// Arbitrary key/value tags for organizing and filtering workflow runs
    #[serde(rename = "tags", skip_serializing_if = "Option::is_none")]
    pub tags: Option<std::collections::HashMap<String, String>>,

    /// Additional parameters to pass to the workflow engine
    #[serde(rename = "workflow_engine_parameters", skip_serializing_if = "Option::is_none")]
    pub workflow_engine_parameters: Option<Vec<ValidatedParam>>,

    /// The workflow engine, must be one supported by this WES instance.
    /// Required if workflow_engine_version is provided.
    #[serde(rename = "workflow_engine", skip_serializing_if = "Option::is_none")]
    pub workflow_engine: Option<String>,

    /// The workflow engine version, must be one supported by this WES instance.
    /// If workflow_engine is provided, but workflow_engine_version is not,
    /// servers can make no assumptions with regard to the engine version
    /// the WES instance uses to process the request if that WES instance
    /// supports multiple versions of the requested engine.
    #[serde(rename = "workflow_engine_version", skip_serializing_if = "Option::is_none")]
    pub workflow_engine_version: Option<String>,

    /// **REQUIRED** The workflow CWL or WDL document. When
    /// `workflow_attachments` is used to attach files, the `workflow_url`
    /// may be a relative path to one of the attachments.
    #[serde(rename = "workflow_url")]
    pub workflow_url: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowFileInfo {
    /// The name of the file.
    pub name: String,

    /// The URL of the file.
    /// Example: "file:///path/to/file"
    ///     "s3://bucket/key"
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunRequestMessage {
    pub request: ValidatedRunRequest,
    pub user_id: String,
}
