use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use common::configs::{
    EngineConfig, FullEngineConfig, RunConfigTemplate, RunsConfig, WorkdirConfig,
};
use common::models::RunRequest;
use common::validators::EngineRequestValidator;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::clients::Db;
use crate::config::ServerConfig;
use crate::engine::Engine;
use crate::error::{EngineError, EngineResult};
use crate::runtime::EngineRuntime;

#[derive(Parser)]
#[command(about = "Metis Workflow Execution Engine")]
#[command(version)]
struct Cli {
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Enable JSON logging
    #[arg(long)]
    json_logging: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single workflow
    Run {
        /// WES request as JSON string
        #[arg(value_name = "WES_REQUEST")]
        request: Option<String>,

        /// Path to WES request JSON file
        #[arg(long, short = 'f', value_name = "FILE")]
        file: Option<PathBuf>,

        /// Engine configuration file
        #[arg(long, value_name = "FILE")]
        engine_config: Option<PathBuf>,

        /// Perform a dry run (validate without executing)
        #[arg(long)]
        dry_run: bool,
    },
    /// Start the engine server
    Server {
        /// Engine configuration file
        #[arg(long, value_name = "FILE", default_value = "/etc/metis/engine-config.yaml")]
        engine_config: PathBuf,
    },
}

/// Parse CLI args, init logging, and dispatch to `run` or `server` subcommand.
pub async fn run_cli<E: Engine + 'static>(engine: E) -> EngineResult<()> {
    let cli = Cli::parse();

    init_logging(&cli.log_level, cli.json_logging)?;

    info!("Starting Metis Engine");

    match cli.command {
        Commands::Run { request, file, engine_config, dry_run } => {
            run_single_workflow(engine, request, file, engine_config, dry_run).await?;
        },
        Commands::Server { engine_config } => {
            run_server(engine, engine_config).await?;
        },
    }

    Ok(())
}

pub fn init_logging(log_level: &str, json_logging: bool) -> EngineResult<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("Invalid log level: {}, defaulting to info", log_level);
            tracing::Level::INFO
        },
    };

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if json_logging {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    Ok(())
}

pub async fn load_engine_config(config_path: Option<PathBuf>) -> EngineResult<FullEngineConfig> {
    match config_path {
        Some(path) => {
            info!(config_path = %path.display(), "Loading engine configuration");

            let config_content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read config file {}: {}", path.display(), e))?;

            match serde_yaml::from_str::<FullEngineConfig>(&config_content) {
                Ok(config) => {
                    info!(
                        engine = %config.engine.name,
                        version = %config.engine.version,
                        "Loaded full engine configuration"
                    );
                    Ok(config)
                },
                Err(full_err) => {
                    let engine_config: EngineConfig = serde_yaml::from_str(&config_content)
                        .map_err(|engine_err| {
                            format!(
                                "Failed to parse config file {} as FullEngineConfig ({}) or EngineConfig ({})",
                                path.display(),
                                full_err,
                                engine_err
                            )
                        })?;

                    warn!(
                        "Loaded legacy flat EngineConfig from {}; wrapping it with default runs config",
                        path.display()
                    );
                    Ok(wrap_engine_config(engine_config))
                },
            }
        },
        None => Err(EngineError::Config(
            "No engine configuration file provided. Use --engine-config <path>".to_string(),
        )),
    }
}

pub fn wrap_engine_config(engine: EngineConfig) -> FullEngineConfig {
    FullEngineConfig {
        version: "1.0.0".to_string(),
        engine,
        runs: RunsConfig {
            workdir: WorkdirConfig {
                base: "/tmp/wes_runs".to_string(),
                pattern: "{user_id}/{run_id}".to_string(),
                subdirs: Some(HashMap::from([
                    ("logs".to_string(), "logs".to_string()),
                    ("outputs".to_string(), "outputs".to_string()),
                    ("work".to_string(), "work".to_string()),
                ])),
            },
            config: Some(RunConfigTemplate {
                filepath: "{run_id}.yaml".to_string(),
                content: "# default per-run config".to_string(),
            }),
            hooks: None,
        },
    }
}

pub async fn parse_wes_request(
    request_json: Option<String>,
    file_path: Option<PathBuf>,
) -> EngineResult<RunRequest> {
    let json_content = match (request_json, file_path) {
        (Some(json), None) => json,
        (None, Some(path)) => {
            info!(file_path = %path.display(), "Reading WES request from file");
            tokio::fs::read_to_string(&path).await.map_err(|e| {
                EngineError::Config(format!(
                    "Failed to read request file {}: {}",
                    path.display(),
                    e
                ))
            })?
        },
        (Some(_), Some(_)) => {
            return Err(EngineError::Config(
                "Cannot specify both request JSON and file path".to_string(),
            ));
        },
        (None, None) => {
            return Err(EngineError::Config(
                "Must specify either request JSON or file path".to_string(),
            ));
        },
    };

    let request: RunRequest = serde_json::from_str(&json_content)
        .map_err(|e| EngineError::Validation(format!("Failed to parse WES request JSON: {}", e)))?;

    info!(
        workflow_type = %request.workflow_type,
        workflow_url = %request.workflow_url,
        "Parsed WES request"
    );

    Ok(request)
}

async fn run_single_workflow<E: Engine + 'static>(
    engine: E,
    request_json: Option<String>,
    file_path: Option<PathBuf>,
    engine_config_path: Option<PathBuf>,
    dry_run: bool,
) -> EngineResult<()> {
    info!("Running single workflow execution");

    let config = load_engine_config(engine_config_path).await?;
    let request = parse_wes_request(request_json, file_path).await?;

    info!(
        engine = %config.engine.name,
        version = %config.engine.version,
        "Loaded engine configuration"
    );

    let validator = EngineRequestValidator::new(config.engine.clone());
    let validated_request = validator
        .validate(&request)
        .map_err(|e| EngineError::Validation(format!("Request validation failed: {}", e)))?;

    info!("WES request validation passed");

    let server_cfg = ServerConfig::from_env().unwrap_or_default();
    let db: Option<Arc<Db>> = match &server_cfg.database_url {
        Some(url) if !url.is_empty() => match Db::new(url).await {
            Ok(db) => Some(Arc::new(db)),
            Err(e) => {
                warn!(error = %e, "Database unavailable — run persistence disabled");
                None
            },
        },
        _ => {
            warn!("DATABASE_URL not set — run persistence disabled");
            None
        },
    };

    let runtime = EngineRuntime::new(Arc::new(engine), config, None, None, db, dry_run);

    let run_id = Uuid::now_v7();
    match runtime.run(run_id, "cli".to_string(), validated_request).await {
        Ok(summary) => {
            info!(
                run_id = %summary.run_id,
                state = ?summary.state,
                "Workflow execution finished"
            );
        },
        Err(e) => {
            if let Some(db) = runtime.db()
                && let Err(db_err) = db
                    .finalize_run(
                        &run_id.to_string(),
                        common::models::State::SystemError,
                        chrono::Utc::now(),
                    )
                    .await
            {
                warn!(run_id = %run_id, error = %db_err, "Failed to record failed run in database");
            }
            return Err(e);
        },
    }
    Ok(())
}

async fn run_server<E: Engine + 'static>(
    engine: E,
    engine_config_path: PathBuf,
) -> EngineResult<()> {
    info!("Starting engine server");

    let config = load_engine_config(Some(engine_config_path)).await?;
    let server_config = ServerConfig::from_env().map_err(|e| {
        EngineError::Config(format!("Failed to load server config from env: {}", e))
    })?;

    info!(
        engine = %config.engine.name,
        version = %config.engine.version,
        nats_url = ?server_config.nats_url,
        redis_url = ?server_config.redis_url,
        database_url = ?server_config.database_url,
        "Engine server configuration loaded"
    );

    if let Err(e) = crate::server::bootstrap(engine, config, server_config).await {
        error!("Server failed: {}", e);
        return Err(EngineError::Execution(format!("Server failed: {}", e)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_parse_wes_request_from_json() {
        let request_json = serde_json::json!({
            "workflow_type": "CWL",
            "workflow_type_version": "v1.0",
            "workflow_url": "https://example.com/workflow.cwl"
        })
        .to_string();

        let request = parse_wes_request(Some(request_json), None).await.unwrap();
        assert_eq!(request.workflow_type, "CWL");
        assert_eq!(request.workflow_url, "https://example.com/workflow.cwl");
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn test_parse_wes_request_from_json_only() {
        let request_json = serde_json::json!({
            "workflow_type": "WDL",
            "workflow_type_version": "1.0",
            "workflow_url": "https://example.com/workflow.wdl"
        })
        .to_string();

        let request = parse_wes_request(Some(request_json), None).await.unwrap();
        assert_eq!(request.workflow_type, "WDL");
    }
}
