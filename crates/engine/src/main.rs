mod clients;
mod command;
mod context;
mod engine;
mod error;
mod models;
mod process;
mod register;
mod runtime;
mod server;
use clap::{Parser, Subcommand};
use common::configs::{
    Backend, EngineConfig, EngineParamsConfig, FullEngineConfig, NatsConfig, RunConfigTemplate,
    RunsConfig, UnknownBehavior, WorkdirConfig, WorkflowParamsConfig, WorkflowParamsMethod,
    WorkflowParamsStyle,
};
use common::models::RunRequest;
use common::validators::EngineRequestValidator;
use engine::NoopEngine;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

fn default_uuid_v7() -> Uuid {
    Uuid::now_v7()
}

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli.log_level, cli.json_logging)?;

    info!("Starting Metis Engine v{}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Run {
            request,
            file,
            engine_config,
        } => {
            run_single_workflow(request, file, engine_config).await?;
        }
        Commands::Server {
            engine_config,
            nats_url,
            notification_subject,
        } => {
            let nats_config = NatsConfig {
                url: nats_url,
                notification_subject,
            };

            run_server(engine_config, nats_config).await?;
        }
    }

    Ok(())
}

/// Initialize logging based on configuration
fn init_logging(log_level: &str, json_logging: bool) -> Result<(), Box<dyn std::error::Error>> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("Invalid log level: {}, defaulting to info", log_level);
            tracing::Level::INFO
        }
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
async fn load_engine_config(
    config_path: Option<PathBuf>,
) -> Result<FullEngineConfig, Box<dyn std::error::Error>> {
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
                }
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
                }
            }
        }
        None => {
            info!("Using default engine configuration");
            Ok(create_default_engine_config())
        }
    }
}

fn create_default_engine() -> EngineConfig {
    EngineConfig {
        name: "default".to_string(),
        version: "1.0.0".to_string(),
        workflow_types: vec!["CWL".to_string(), "WDL".to_string(), "NFL".to_string()],
        workflow_type_versions: vec!["v1.0".to_string(), "1.0".to_string(), "DSL2".to_string()],
        command_template: "echo 'Running workflow: {workflow_path}' && sleep 5".to_string(),
        backend: Backend::Local,
        id: default_uuid_v7(),

        workflow_params: WorkflowParamsConfig {
            style: WorkflowParamsStyle {
                method: WorkflowParamsMethod::Inline,
                prefix: Some("--".to_string()),
                separator: Some(" ".to_string()),
                key_value_format: Some("{key}={value}".to_string()),
                format: None,
                file_path: None,
            },
        },
        engine_params: EngineParamsConfig {
            unknown_params_behavior: UnknownBehavior::Strip,
            validated_params: vec![],
        },
        denied_params: vec![],
        ignored_params: None,
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

fn create_default_engine_config() -> FullEngineConfig {
    wrap_engine_config(create_default_engine())
}

/// Parse WES request from JSON string or file
async fn parse_wes_request(
    request_json: Option<String>,
    file_path: Option<PathBuf>,
) -> Result<RunRequest, Box<dyn std::error::Error>> {
    let json_content = match (request_json, file_path) {
        (Some(json), None) => json,
        (None, Some(path)) => {
            info!(file_path = %path.display(), "Reading WES request from file");
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read request file {}: {}", path.display(), e))?
        }
        (Some(_), Some(_)) => {
            return Err("Cannot specify both request JSON and file path".into());
        }
        (None, None) => {
            return Err("Must specify either request JSON or file path".into());
        }
    };

    let request: RunRequest = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse WES request JSON: {}", e))?;

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
) -> Result<(), Box<dyn std::error::Error>> {
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
        .map_err(|e| format!("Request validation failed: {}", e))?;

    info!("WES request validation passed");

    // Create runtime with dry_run=true for NoopEngine
    let runtime = runtime::EngineRuntime::new(
        Arc::new(NoopEngine),
        config,
        None,  // no NATS
        None,  // no Valkey
        true,  // dry_run = true
    )
    .await?;

    let run_id = Uuid::now_v7();
    let summary = runtime
        .run(run_id, "cli".to_string(), validated_request)
        .await?;

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
) -> Result<(), Box<dyn std::error::Error>> {
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
    if let Err(e) = server::start_server(config, nats_config).await {
        error!("Server failed: {}", e);
        return Err(e);
    }

    Ok(())
}

/// Example WES request for testing
#[allow(dead_code)]
fn example_wes_request() -> serde_json::Value {
    serde_json::json!({
        "workflow_params": {
            "input_file": "/path/to/input.txt",
            "output_dir": "/path/to/output"
        },
        "workflow_type": "CWL",
        "workflow_type_version": "v1.0",
        "workflow_url": "https://github.com/example/workflow.cwl",
        "workflow_engine_parameters": {
            "max_cpu": "4",
            "max_memory": "8GB"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = create_default_engine_config();
        assert_eq!(config.engine.name, "default");
        assert_eq!(config.engine.version, "1.0.0");
        assert!(!config.engine.workflow_types.is_empty());
    }

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

    #[tokio::test]
    async fn test_load_default_config() {
        let result = load_engine_config(None).await;
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.engine.name, "default");
    }
}
