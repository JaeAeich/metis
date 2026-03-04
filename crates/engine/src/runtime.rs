use std::collections::{HashMap, HashSet};
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use common::configs::FullEngineConfig;
use common::models::{RunRequestMessage, RunSummary, State, TaskListResponse, ValidatedRunRequest};
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::clients::db::Stream;
use crate::clients::{Db, Nats, Valkey};
use crate::command::EngineCommandBuilder;
use crate::dry_run::DryRunReport;
use crate::engine::Engine;
use crate::error::{EngineError, EngineResult};
use crate::execution::ProcessExecutor;
use crate::models::{BuildContext, CommandInfo};
use crate::pid_store::PidStore;
use crate::workdir::WorkdirManager;

pub struct ExecutionContext {
    pub run_id: Uuid,
    pub user_id: String,
    pub workflow_path: String,
    pub workflow_url: String,
    pub workdir: String,
    pub subdirs: HashMap<String, String>,
    pub started_at: DateTime<Utc>,
}

pub struct EngineRuntime {
    engine: Arc<dyn Engine>,
    config: Arc<FullEngineConfig>,
    valkey: Option<Arc<Valkey>>,
    nats: Option<Arc<Nats>>,
    db: Option<Arc<Db>>,
    command_builder: Arc<EngineCommandBuilder>,
    pid_store: PidStore,
    cancelled_runs: Arc<RwLock<HashSet<Uuid>>>,
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct CancelRunMessage {
    run_id: String,
}

impl EngineRuntime {
    pub fn new(
        engine: Arc<dyn Engine>,
        config: FullEngineConfig,
        nats: Option<Arc<Nats>>,
        valkey: Option<Arc<Valkey>>,
        db: Option<Arc<Db>>,
        dry_run: bool,
    ) -> Self {
        let command_builder = Arc::new(EngineCommandBuilder::new(Arc::new(config.engine.clone())));
        let pid_store = PidStore::new(valkey.clone());

        Self {
            engine,
            config: Arc::new(config),
            valkey,
            nats,
            db,
            command_builder,
            pid_store,
            cancelled_runs: Arc::new(RwLock::new(HashSet::new())),
            dry_run,
        }
    }

    pub fn db(&self) -> Option<&Arc<Db>> {
        self.db.as_ref()
    }

    pub async fn start(self: Arc<Self>) -> EngineResult<()> {
        if let Some(valkey) = &self.valkey {
            let is_new = valkey.am_i_new().await?;
            info!(
                engine = %self.config.engine.name,
                version = %self.config.engine.version,
                is_new = is_new,
                "Registering engine in Valkey"
            );
            valkey.register().await?;

            let valkey = Arc::clone(valkey);
            tokio::spawn(async move {
                loop {
                    if let Err(e) = valkey.heartbeat().await {
                        warn!(error = %e, "Failed to update engine heartbeat");
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                }
            });
        }

        let Some(nats) = &self.nats else {
            warn!("NATS not configured — server mode disabled, waiting for shutdown signal");
            tokio::signal::ctrl_c().await?;
            info!("Received shutdown signal");
            return Ok(());
        };

        let registration_event = serde_json::json!({
            "event": "engine_registered",
            "engine_id": self.config.engine.id,
            "engine_name": self.config.engine.name,
            "engine_version": self.config.engine.version,
            "timestamp": Utc::now().to_rfc3339(),
        });
        nats.publish_notification(registration_event.to_string().into_bytes()).await?;

        let run_subscribers = nats.subscribe_runs().await?;
        let cancel_subscriber = nats.subscribe_cancel().await?;

        for mut subscriber in run_subscribers {
            let runtime = Arc::clone(&self);
            tokio::spawn(async move {
                while let Some(message) = subscriber.next().await {
                    let runtime = Arc::clone(&runtime);
                    tokio::spawn(async move {
                        if let Err(e) = runtime.handle_run_message(message.payload.as_ref()).await {
                            error!(error = %e, "Failed to handle run message");
                        }
                    });
                }
            });
        }

        {
            let runtime = Arc::clone(&self);
            tokio::spawn(async move {
                let mut subscriber = cancel_subscriber;
                while let Some(message) = subscriber.next().await {
                    let runtime = Arc::clone(&runtime);
                    tokio::spawn(async move {
                        if let Err(e) =
                            runtime.handle_cancel_message(message.payload.as_ref()).await
                        {
                            error!(error = %e, "Failed to handle cancel message");
                        }
                    });
                }
            });
        }

        info!("Engine server is ready and listening for run/cancel requests");
        tokio::signal::ctrl_c().await?;
        info!("Received shutdown signal");
        Ok(())
    }

    async fn handle_run_message(&self, payload: &[u8]) -> EngineResult<()> {
        let run_message = Self::parse_run_message(payload)?;
        let run_id = Uuid::parse_str(&run_message.run_id)
            .map_err(|e| EngineError::Generic(format!("Invalid run_id: {}", e)))?;

        if let Some(valkey) = &self.valkey {
            valkey.add_run(&run_id.to_string()).await?;
        }

        let run_result = self.run(run_id, run_message.user_id.clone(), run_message.request).await;

        if let Some(valkey) = &self.valkey {
            valkey.remove_run(&run_id.to_string()).await?;
        }

        match run_result {
            Ok(summary) => {
                if let Some(nats) = &self.nats {
                    let event = serde_json::json!({
                        "event": "run_completed",
                        "run_id": summary.run_id,
                        "state": summary.state,
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    nats.publish_notification(event.to_string().into_bytes()).await?;
                }
            },
            Err(e) => {
                if let Some(db) = &self.db
                    && let Err(db_err) =
                        db.finalize_run(&run_id.to_string(), State::SystemError, Utc::now()).await
                {
                    warn!(run_id = %run_id, error = %db_err, "Failed to record failed run in database");
                }
                if let Some(nats) = &self.nats {
                    let event = serde_json::json!({
                        "event": "run_failed",
                        "run_id": run_id,
                        "error": e.to_string(),
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    nats.publish_notification(event.to_string().into_bytes()).await?;
                }
                return Err(e);
            },
        }

        Ok(())
    }

    async fn handle_cancel_message(&self, payload: &[u8]) -> EngineResult<()> {
        let run_id = Self::parse_cancel_run_id(payload)?;
        self.cancel(&run_id).await?;

        if let Some(nats) = &self.nats {
            let event = serde_json::json!({
                "event": "run_canceled",
                "run_id": run_id,
                "timestamp": Utc::now().to_rfc3339(),
            });
            nats.publish_notification(event.to_string().into_bytes()).await?;
        }

        Ok(())
    }

    fn parse_run_message(payload: &[u8]) -> EngineResult<RunRequestMessage> {
        if let Ok(message) = serde_json::from_slice::<RunRequestMessage>(payload) {
            return Ok(message);
        }

        let request: ValidatedRunRequest = serde_json::from_slice(payload).map_err(|e| {
            EngineError::Generic(format!(
                "Unable to parse run message as RunRequestMessage or ValidatedRunRequest: {}",
                e
            ))
        })?;

        Ok(RunRequestMessage {
            run_id: Uuid::now_v7().to_string(),
            request,
            user_id: "default".to_string(),
        })
    }

    fn parse_cancel_run_id(payload: &[u8]) -> EngineResult<String> {
        if let Ok(message) = serde_json::from_slice::<CancelRunMessage>(payload) {
            return Ok(message.run_id);
        }

        if let Ok(raw) = serde_json::from_slice::<String>(payload) {
            return Ok(raw);
        }

        let raw_text = std::str::from_utf8(payload).map_err(|e| {
            EngineError::Generic(format!(
                "Unable to parse cancel message payload as UTF-8 string: {}",
                e
            ))
        })?;

        if raw_text.is_empty() {
            return Err(EngineError::Generic("Cancel payload is empty".to_string()));
        }

        Ok(raw_text.to_string())
    }

    /// Main workflow execution method
    pub async fn run(
        &self,
        run_id: Uuid,
        user_id: String,
        req: ValidatedRunRequest,
    ) -> Result<RunSummary, EngineError> {
        let start_time = Instant::now();
        let started_at = Utc::now();

        let base_context = WorkdirManager::create(&self.config, &run_id.to_string(), &user_id)?;
        let ctx = ExecutionContext {
            run_id,
            user_id: user_id.clone(),
            workflow_path: req.workflow_url.clone(),
            workflow_url: req.workflow_url.clone(),
            workdir: base_context.workdir.clone(),
            subdirs: base_context.subdirs.clone(),
            started_at,
        };

        let span = tracing::info_span!(
            "workflow_run",
            run_id = %ctx.run_id,
            workflow_type = %req.workflow_type,
            state = tracing::field::Empty,
            duration_ms = tracing::field::Empty,
        );

        info!(
            run_id = %ctx.run_id,
            workflow_type = %req.workflow_type,
            workflow_url = %req.workflow_url,
            dry_run = self.dry_run,
            "Starting workflow execution"
        );

        // Step 1: Setup execution environment
        span.record("state", "initializing");
        WorkdirManager::setup(&ctx.workdir).await?;

        if let Some(db) = &self.db
            && let Err(e) =
                db.insert_run(&ctx.run_id.to_string(), &ctx.user_id, &req, started_at).await
        {
            warn!(run_id = %ctx.run_id, error = %e, "Failed to insert run into database");
        }

        // Step 2: Build execution command
        span.record("state", "building");
        let build_context = self.create_build_context(&ctx).await?;

        let engine_params = self
            .command_builder
            .build_engine_params_string(req.workflow_engine_parameters.as_ref(), &build_context)
            .map_err(|e| EngineError::Generic(e.to_string()))?;

        let workflow_params = self
            .command_builder
            .build_workflow_params_string(req.workflow_params.as_ref(), &build_context)
            .map_err(|e| EngineError::Generic(e.to_string()))?;

        let mut build_context = build_context;
        build_context.engine_params = engine_params;
        build_context.workflow_params = workflow_params;

        let command_info = self.build_command(&req, &build_context).await?;

        info!(
            command = %command_info.redacted_command,
            workdir = %command_info.workdir,
            "Built execution command"
        );

        // DRY RUN MODE: Print command and exit
        if self.dry_run {
            let tags = req.tags.clone();

            DryRunReport {
                run_id: ctx.run_id,
                user_id: ctx.user_id.clone(),
                workflow_url: ctx.workflow_url.clone(),
                workflow_type: req.workflow_type.clone(),
                workflow_type_version: req.workflow_type_version.clone(),
                workdir: ctx.workdir.clone(),
                subdirs: ctx.subdirs.clone(),
                command_info,
                request: req,
            }
            .print();

            return Ok(RunSummary {
                run_id: ctx.run_id.to_string(),
                state: Some(State::Complete),
                tags: tags.unwrap_or_default(),
                start_time: Some(ctx.started_at.to_rfc3339()),
                end_time: Some(Utc::now().to_rfc3339()),
            });
        }

        // Step 3: Execute workflow process
        span.record("state", "executing");
        let child_process = ProcessExecutor::execute(&command_info).await?;
        let pid = child_process
            .id()
            .ok_or_else(|| EngineError::Execution("Failed to get process ID".to_string()))?;

        // Store PID for cancellation support
        self.pid_store.store(&ctx.run_id.to_string(), pid).await?;
        info!(pid = pid, "Workflow process started");

        if let Some(db) = &self.db
            && let Err(e) = db.update_run_state(&ctx.run_id.to_string(), State::Running).await
        {
            warn!(run_id = %ctx.run_id, error = %e, "Failed to update run state to RUNNING");
        }

        // Step 4: Monitor execution (this blocks until completion or cancellation)
        span.record("state", "running");
        let line_callback: Arc<dyn Fn(Stream, u64, String) + Send + Sync + 'static> = {
            let db = self.db.clone();
            let run_id = ctx.run_id.to_string();
            Arc::new(move |stream, seq, line| {
                if let Some(db) = db.clone() {
                    let run_id = run_id.clone();
                    tokio::spawn(async move {
                        if let Err(e) = db.insert_log_line(&run_id, stream, seq, &line).await {
                            warn!(run_id = %run_id, seq = seq, error = %e, "Failed to stream log line to DB");
                        }
                    });
                }
            })
        };
        let execution_output = ProcessExecutor::monitor(child_process, line_callback).await?;
        let exit_status = execution_output.exit_status;

        let is_cancelled = self.is_run_cancelled(&ctx.run_id).await;

        // Step 5: Parse results regardless of success/failure/cancellation
        span.record("state", "parsing");
        let (run_summary, run_log, task_logs) =
            self.parse_execution_results(&ctx, &exit_status, is_cancelled).await?;

        // Step 6: Update database with results
        span.record("state", "updating_db");
        self.update_database(&ctx, &run_summary, &run_log, &task_logs).await?;

        // Step 7: Cleanup
        span.record("state", "cleanup");
        self.cleanup_execution(&ctx).await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        span.record("duration_ms", duration_ms);
        span.record("state", format!("{:?}", run_summary.state).as_str());

        info!(
            duration_ms = duration_ms,
            state = ?run_summary.state,
            "Workflow execution completed"
        );

        Ok(run_summary)
    }

    /// Cancel a running workflow - FINAL, cannot be overridden
    pub async fn cancel(&self, run_id: &str) -> Result<(), EngineError> {
        let _span = tracing::info_span!("workflow_cancel", run_id = %run_id);
        info!(run_id = %run_id, "Cancelling workflow");

        if let Ok(parsed) = Uuid::parse_str(run_id) {
            self.cancelled_runs.write().await.insert(parsed);
        }

        let pid = match self.pid_store.get(run_id).await? {
            Some(pid) => pid,
            None => {
                warn!("No PID found for run_id, workflow may have already completed");
                return Ok(());
            },
        };

        info!(pid = pid, "Found process to cancel");

        // Send cancellation signal
        ProcessExecutor::cancel(pid).await?;
        self.pid_store.remove(run_id).await?;

        info!("Workflow cancellation completed");
        Ok(())
    }

    async fn is_run_cancelled(&self, run_id: &Uuid) -> bool {
        self.cancelled_runs.read().await.contains(run_id)
    }

    /// Create build context for command generation - FINAL, cannot be
    /// overridden
    async fn create_build_context(
        &self,
        ctx: &ExecutionContext,
    ) -> Result<BuildContext, EngineError> {
        Ok(BuildContext {
            run_id: ctx.run_id.to_string(),
            user_id: ctx.user_id.clone(),
            workflow_path: ctx.workflow_path.clone(),
            workflow_url: ctx.workflow_url.clone(),
            workdir: ctx.workdir.clone(),
            subdirs: ctx.subdirs.clone(),
            timestamp: chrono::Utc::now(),
            workflow_params: String::new(),
            engine_params: String::new(),
            params_file: format!("{}/workflow-params.json", ctx.workdir),
            log_dir: ctx.subdirs.get("logs").cloned().unwrap_or_default(),
            output_dir: ctx.subdirs.get("outputs").cloned().unwrap_or_default(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            time: chrono::Utc::now().format("%H-%M-%S").to_string(),
        })
    }

    /// Build the execution command - FINAL, cannot be overridden
    async fn build_command(
        &self,
        request: &ValidatedRunRequest,
        context: &BuildContext,
    ) -> Result<CommandInfo, EngineError> {
        self.command_builder
            .build_command(request, context)
            .map_err(|e| EngineError::Generic(e.to_string()))
    }

    /// Execute the workflow command - FINAL, cannot be overridden
    /// Parse execution results and delegate to engine-specific methods - FINAL,
    /// cannot be overridden
    async fn parse_execution_results(
        &self,
        ctx: &ExecutionContext,
        exit_status: &ExitStatus,
        is_cancelled: bool,
    ) -> Result<(RunSummary, common::models::Log, TaskListResponse), EngineError> {
        let state = if is_cancelled {
            State::Canceled
        } else if exit_status.success() {
            State::Complete
        } else {
            State::ExecutorError
        };

        // Delegate to engine-specific parsing methods
        let run_summary = self.parse_run_summary(ctx, &state).await?;
        let run_log = self.parse_run_log(ctx, &state, exit_status).await?;
        let task_logs = self.parse_task_logs(ctx).await?;

        Ok((run_summary, run_log, task_logs))
    }

    /// Update database with execution results - FINAL, cannot be overridden
    async fn update_database(
        &self,
        ctx: &ExecutionContext,
        run_summary: &RunSummary,
        run_log: &common::models::Log,
        task_logs: &TaskListResponse,
    ) -> Result<(), EngineError> {
        let Some(db) = &self.db else {
            return Ok(());
        };

        let state = run_summary.state.unwrap_or(State::Unknown);
        let end_time = Utc::now();

        if let Err(e) = db.finalize_run(&ctx.run_id.to_string(), state, end_time).await {
            warn!(run_id = %ctx.run_id, error = %e, "Failed to finalize run in database");
        }

        if let Err(e) = db.insert_run_log(&ctx.run_id.to_string(), run_log).await {
            warn!(run_id = %ctx.run_id, error = %e, "Failed to insert run log");
        }

        if let Some(tasks) = &task_logs.task_logs
            && let Err(e) = db.insert_task_logs(&ctx.run_id.to_string(), tasks).await
        {
            warn!(run_id = %ctx.run_id, error = %e, "Failed to insert task logs");
        }

        Ok(())
    }

    /// Cleanup execution environment - FINAL, cannot be overridden
    async fn cleanup_execution(&self, ctx: &ExecutionContext) -> Result<(), EngineError> {
        info!(run_id = %ctx.run_id, "Cleaning up execution environment");

        self.pid_store.remove(&ctx.run_id.to_string()).await?;
        self.cancelled_runs.write().await.remove(&ctx.run_id);

        // TODO: Optionally clean up working directory based on config
        // TODO: Archive logs if configured
        Ok(())
    }

    async fn parse_run_summary(
        &self,
        ctx: &ExecutionContext,
        state: &State,
    ) -> Result<RunSummary, EngineError> {
        Ok(RunSummary {
            run_id: ctx.run_id.to_string(),
            state: Some(*state),
            tags: HashMap::new(),
            start_time: Some(ctx.started_at.to_rfc3339()),
            end_time: Some(Utc::now().to_rfc3339()),
        })
    }

    async fn parse_run_log(
        &self,
        ctx: &ExecutionContext,
        _state: &State,
        exit_status: &ExitStatus,
    ) -> Result<common::models::Log, EngineError> {
        Ok(common::models::Log {
            name: Some(format!("run-{}", ctx.run_id)),
            cmd: None,
            start_time: Some(ctx.started_at.to_rfc3339()),
            end_time: Some(Utc::now().to_rfc3339()),
            stdout: None,
            stderr: None,
            exit_code: exit_status.code(),
            system_logs: None,
        })
    }

    async fn parse_task_logs(
        &self,
        _ctx: &ExecutionContext,
    ) -> Result<TaskListResponse, EngineError> {
        let task_logs = self.engine.get_task_logs().await?.unwrap_or_default();

        Ok(TaskListResponse { task_logs: Some(task_logs), next_page_token: None })
    }
}
