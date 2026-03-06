use serde::Deserialize;
use url::Url;

use crate::error::{EngineError, EngineResult};

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub database_url: Url,
    pub nats_url: Url,
    #[serde(default = "default_notification_subject")]
    pub nats_notification_subject: String,
    pub redis_url: Url,
}

fn default_notification_subject() -> String {
    common::keys::nats_notification_subject().to_string()
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}

pub async fn load_engine_config() -> EngineResult<common::configs::FullEngineConfig> {
    let config_path = match std::env::var("ENGINE_CONFIG_PATH") {
        Ok(path) => path,
        Err(_) => {
            tracing::warn!(
                "ENGINE_CONFIG_PATH not set, defaulting to /etc/metis/engine-config.yaml"
            );
            "/etc/metis/engine-config.yaml".to_string()
        },
    };

    let config_content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
        EngineError::Config(format!("Failed to read config file {}: {}", config_path, e))
    })?;

    serde_yaml::from_str(&config_content).map_err(|e| {
        EngineError::Config(format!("Failed to parse config file {}: {}", config_path, e))
    })
}
