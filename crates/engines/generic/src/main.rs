use std::collections::HashMap;

use async_trait::async_trait;
use common::configs::FullEngineConfig;
use common::models::{TaskLog, WorkflowFileInfo};
use engine::config::ServerConfig;
use engine::{Engine, EngineError, EngineResult, server};

struct GenericEngine;

#[async_trait]
impl Engine for GenericEngine {
    fn new() -> Self {
        Self
    }

    async fn get_workflow_results(
        &self,
    ) -> Result<Option<HashMap<String, HashMap<String, WorkflowFileInfo>>>, EngineError> {
        Ok(None)
    }

    async fn get_task_logs(&self) -> Result<Option<Vec<TaskLog>>, EngineError> {
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> EngineResult<()> {
    let engine = GenericEngine;
    let config: FullEngineConfig = load_config().await?;
    let server_config = ServerConfig::from_env()
        .map_err(|e| EngineError::Config(format!("Failed to load server config: {}", e)))?;
    server::bootstrap(engine, config, server_config).await
}

async fn load_config() -> EngineResult<FullEngineConfig> {
    let config_path = std::env::var("ENGINE_CONFIG_PATH")
        .unwrap_or_else(|_| "/etc/metis/engine-config.yaml".to_string());

    let config_content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
        EngineError::Config(format!("Failed to read config file {}: {}", config_path, e))
    })?;

    serde_yaml::from_str(&config_content)
        .map_err(|e| EngineError::Config(format!("Failed to parse config file: {}", e)))
}
