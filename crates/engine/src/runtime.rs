use std::collections::{HashMap, HashSet};
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};
use common::configs::{FullEngineConfig, NatsConfig, ValkeyConfig};
use common::models::{RunRequestMessage, RunSummary, State, TaskListResponse, ValidatedRunRequest};
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::clients::{Nats, Valkey};
use crate::command::EngineCommandBuilder;
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
    pub cancelled: Arc<AtomicBool>,
}

pub struct EngineRuntime {
    engine: Arc<dyn Engine>,
    config: Arc<FullEngineConfig>,
    valkey: Option<Arc<Valkey>>,
    nats: Option<Arc<Nats>>,
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
    pub async fn new(
        engine: Arc<dyn Engine>,
        config: FullEngineConfig,
        nats_config: Option<NatsConfig>,
        valkey_config: Option<ValkeyConfig>,
        dry_run: bool,
    ) -> EngineResult<Self> {
        let command_builder = Arc::new(EngineCommandBuilder::new(Arc::new(config.engine.clone())));

        let valkey = if let Some(valkey_config) = valkey_config {
            Some(Arc::new(Valkey::new(valkey_config, config.engine.clone(), 60).await?))
        } else {
            None
        };

        let nats = if let Some(nats_config) = nats_config {
            Some(Arc::new(Nats::new(&nats_config, config.engine.clone()).await?))
        } else {
            None
        };

        let pid_store = PidStore::new(valkey.clone());

        Ok(Self {
            engine,
            config: Arc::new(config),
            valkey,
            nats,
            command_builder,
            pid_store,
            cancelled_runs: Arc::new(RwLock::new(HashSet::new())),
            dry_run,
        })
    }

    pub async fn start(self: Arc<Self>) -> EngineResult<()> {
        let Some(nats) = &self.nats else {
            return Err(EngineError::Generic(
                "NATS is not configured for this runtime instance".to_string(),
            ));
        };

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
        let run_id = Uuid::now_v7();

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

        Ok(RunRequestMessage { request, user_id: "default".to_string() })
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
            cancelled: Arc::new(AtomicBool::new(false)),
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
            self.print_dry_run_summary(&ctx, &command_info, &req);

            return Ok(RunSummary {
                run_id: ctx.run_id.to_string(),
                state: Some(State::Complete),
                tags: req.tags.unwrap_or_default(),
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

        // Step 4: Monitor execution (this blocks until completion or cancellation)
        span.record("state", "running");
        let execution_output = ProcessExecutor::monitor(child_process).await?;
        let exit_status = execution_output.exit_status;

        if self.is_run_cancelled(&ctx.run_id).await {
            ctx.cancelled.store(true, Ordering::Relaxed);
        }

        // Step 5: Parse results regardless of success/failure/cancellation
        span.record("state", "parsing");
        let (run_summary, run_log, task_logs) =
            self.parse_execution_results(&ctx, &exit_status).await?;

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

    fn print_dry_run_summary(
        &self,
        ctx: &ExecutionContext,
        command_info: &CommandInfo,
        req: &ValidatedRunRequest,
    ) {
        println!();
        println!("{}", "═".repeat(70));
        println!("DRY RUN - NoopEngine");
        println!("{}", "═".repeat(70));
        println!();
        println!("Run ID:         {}", ctx.run_id);
        println!("User ID:        {}", ctx.user_id);
        println!("Workflow URL:   {}", ctx.workflow_url);
        println!("Workflow Type:  {} {}", req.workflow_type, req.workflow_type_version);
        println!();
        println!("Staging Area:");
        println!("  workdir:   {}", ctx.workdir);
        for (name, path) in &ctx.subdirs {
            println!("  {}:   {}", name, path);
        }
        println!();
        println!("{}", "─".repeat(70));
        println!("COMMAND (would execute):");
        println!("{}", "─".repeat(70));
        println!();

        // Print command with nice formatting
        let cmd = &command_info.command;
        if cmd.len() > 80 {
            // Try to break at logical points
            let parts: Vec<&str> = cmd.split(" \\").collect();
            if parts.len() > 1 {
                println!("cd {} && \\", command_info.workdir);
                for part in parts {
                    println!("  {}", part.trim());
                }
            } else {
                println!("cd {} && {}", command_info.workdir, cmd);
            }
        } else {
            println!("cd {} && {}", command_info.workdir, cmd);
        }

        // Print environment variables if any
        if !command_info.env_vars.is_empty() {
            println!();
            println!("Environment Variables:");
            for (k, v) in &command_info.env_vars {
                // Check if this is a sensitive value
                let is_sensitive = req
                    .workflow_engine_parameters
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .any(|p| p.spec.env_var.as_deref() == Some(k) && p.spec.sensitive)
                    })
                    .unwrap_or(false);

                if is_sensitive {
                    println!("  {}=***REDACTED***", k);
                } else {
                    println!("  {}={}", k, v);
                }
            }
        }

        // Print workflow params
        if let Some(ref params) = req.workflow_params {
            println!();
            println!("Workflow Parameters:");
            if let Some(obj) = params.as_object() {
                for (k, v) in obj {
                    println!("  {} = {}", k, v);
                }
            }
        }

        // Print engine params
        if let Some(ref params) = req.workflow_engine_parameters {
            println!();
            println!("Engine Parameters:");
            for p in params {
                let value_str = match &p.value {
                    Some(v) => v.to_string(),
                    None => "(default)".to_string(),
                };
                let sensitive_marker = if p.spec.sensitive { " [SENSITIVE]" } else { "" };
                println!(
                    "  {} = {}{}",
                    p.spec.names.first().unwrap_or(&String::new()),
                    value_str,
                    sensitive_marker
                );
            }
        }

        println!();
        println!("{}", "═".repeat(70));
        println!("DRY RUN COMPLETE - No actual execution performed");
        println!("{}", "═".repeat(70));
        println!();
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
    ) -> Result<(RunSummary, common::models::Log, TaskListResponse), EngineError> {
        let state = if ctx.cancelled.load(Ordering::Relaxed) {
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
        _run_summary: &RunSummary,
        _run_log: &common::models::Log,
        _task_logs: &TaskListResponse,
    ) -> Result<(), EngineError> {
        info!(run_id = %ctx.run_id, "Updating database with execution results");

        // TODO: Implement database updates
        // This would typically:
        // 1. Insert/update run summary in runs table
        // 2. Insert/update run log in run_logs table
        // 3. Insert task logs in task_logs table
        // 4. Update any other relevant tables

        info!("Database update completed (TODO: implement actual DB operations)");
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

    pub fn create_workdir(
        config: &FullEngineConfig,
        run_id: &str,
        user_id: &str,
    ) -> Result<BuildContext, std::io::Error> {
        let timestamp = Utc::now();
        let safe_user_id = user_id.replace('/', "_");

        let pattern = config
            .runs
            .workdir
            .pattern
            .replace("{run_id}", run_id)
            .replace("{user_id}", &safe_user_id)
            .replace("{date}", &timestamp.format("%Y-%m-%d").to_string())
            .replace("{time}", &timestamp.format("%H-%M-%S").to_string())
            .replace("{timestamp}", &timestamp.timestamp().to_string());

        let workdir = format!("{}/{}", config.runs.workdir.base, pattern);
        std::fs::create_dir_all(&workdir)?;

        // Create subdirectories
        let mut subdirs = HashMap::new();
        if let Some(ref subdir_config) = config.runs.workdir.subdirs {
            for (name, subdir) in subdir_config {
                let full_path = format!("{}/{}", workdir, subdir);
                std::fs::create_dir_all(&full_path)?;
                subdirs.insert(name.clone(), full_path);
            }
        }

        Ok(BuildContext {
            run_id: run_id.to_string(),
            user_id: safe_user_id,
            workflow_path: String::new(),
            workflow_url: String::new(),
            workdir,
            subdirs: subdirs.clone(),
            timestamp,
            workflow_params: String::new(),
            engine_params: String::new(),
            params_file: String::new(),
            log_dir: subdirs.get("logs").cloned().unwrap_or_default(),
            output_dir: subdirs.get("outputs").cloned().unwrap_or_default(),
            date: timestamp.format("%Y-%m-%d").to_string(),
            time: timestamp.format("%H-%M-%S").to_string(),
        })
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
