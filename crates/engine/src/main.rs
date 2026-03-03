use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use common::configs::{
    EngineConfig, FullEngineConfig, NatsConfig, RunConfigTemplate, RunsConfig, WorkdirConfig,
};
use common::models::RunRequest;
use common::validators::EngineRequestValidator;
use engine::{EngineError, EngineResult, EngineRuntime, NoopEngine};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "metis-engine")]
#[command(about = "Metis Workflow Execution Engine")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Configuration file path
    #[arg(long, short = 'c', value_name = "FILE")]
    config: Option<PathBuf>,

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
    },
    /// Start the engine server
    Server {
        /// Engine configuration file
        #[arg(long, value_name = "FILE")]
        engine_config: Option<PathBuf>,

        /// NATS server URL
        #[arg(long, default_value = "nats://localhost:4222")]
        nats_url: String,

        /// NATS notification subject
        #[arg(long, default_value = "metis.notification")]
        notification_subject: String,
    },
}

#[tokio::main]
async fn main() -> EngineResult<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli.log_level, cli.json_logging)?;

    info!("Starting Metis Engine v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Run { request, file, engine_config } => {
            run_single_workflow(request, file, engine_config).await?;
        },
        Commands::Server { engine_config, nats_url, notification_subject } => {
            let nats_config = NatsConfig { url: nats_url, notification_subject };

            run_server(engine_config, nats_config).await?;
        },
    }

    Ok(())
}

/// Initialize logging based on configuration
fn init_logging(log_level: &str, json_logging: bool) -> EngineResult<()> {
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

/// Load engine configuration from file or use default
async fn load_engine_config(config_path: Option<PathBuf>) -> EngineResult<FullEngineConfig> {
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

fn wrap_engine_config(engine: EngineConfig) -> FullEngineConfig {
    FullEngineConfig {
        version: "1.0.0".to_string(),
        engine,
        runs: RunsConfig {
            workdir: WorkdirConfig {
                base: "/tmp/wes_runs".to_string(),
                pattern: "{user_id}/{run_id}".to_string(),
                subdirs: Some(std::collections::HashMap::from([
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

/// Parse WES request from JSON string or file
async fn parse_wes_request(
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

/// Run a single workflow execution
async fn run_single_workflow(
    request_json: Option<String>,
    file_path: Option<PathBuf>,
    engine_config_path: Option<PathBuf>,
) -> EngineResult<()> {
    info!("Running single workflow execution");

    // Load configuration and request
    let config = load_engine_config(engine_config_path).await?;
    let request = parse_wes_request(request_json, file_path).await?;

    info!(
        engine = %config.engine.name,
        version = %config.engine.version,
        "Loaded engine configuration"
    );

    // Validate request
    let validator = EngineRequestValidator::new(config.engine.clone());
    let validated_request = validator
        .validate(&request)
        .map_err(|e| EngineError::Validation(format!("Request validation failed: {}", e)))?;

    info!("WES request validation passed");

    // Create runtime with dry_run=true for NoopEngine
    let runtime = EngineRuntime::new(
        Arc::new(NoopEngine),
        config,
        None, // no NATS
        None, // no Valkey
        true, // dry_run = true
    )
    .await?;

    let run_id = Uuid::now_v7();
    let summary = runtime.run(run_id, "cli".to_string(), validated_request).await?;

    info!(
        run_id = %summary.run_id,
        state = ?summary.state,
        "Workflow execution finished"
    );
    Ok(())
}

/// Run the engine server (NATS consumer)
async fn run_server(
    engine_config_path: Option<PathBuf>,
    nats_config: NatsConfig,
) -> EngineResult<()> {
    info!("Starting engine server");

    // Load engine configuration
    let config = load_engine_config(engine_config_path).await?;

    info!(
        engine = %config.engine.name,
        version = %config.engine.version,
        nats_url = %nats_config.url,
        notification_subject = %nats_config.notification_subject,
        "Engine server configuration loaded"
    );

    // Start the server
    if let Err(e) = engine::server::start_server(config, nats_config).await {
        error!("Server failed: {}", e);
        return Err(EngineError::Execution(format!("Server failed: {}", e)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_wes_request_from_json() {
        let request_json = serde_json::json!({
            "workflow_type": "CWL",
            "workflow_type_version": "v1.0",
            "workflow_url": "https://example.com/workflow.cwl"
        })
        .to_string();

        let result = parse_wes_request(Some(request_json), None).await;
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.workflow_type, "CWL");
        assert_eq!(request.workflow_url, "https://example.com/workflow.cwl");
    }

    #[tokio::test]
    async fn test_parse_wes_request_from_json_only() {
        // Test with invalid file path to ensure JSON parsing works
        let request_json = serde_json::json!({
            "workflow_type": "WDL",
            "workflow_type_version": "1.0",
            "workflow_url": "https://example.com/workflow.wdl"
        })
        .to_string();

        let result = parse_wes_request(Some(request_json), None).await;
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.workflow_type, "WDL");
    }
}
