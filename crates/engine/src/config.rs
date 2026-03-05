use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub database_url: Url,
    pub nats_url: Url,
    #[serde(default = "default_notification_subject")]
    pub nats_notification_subject: String,
    pub redis_url: Url,
}

fn default_notification_subject() -> String {
    "metis.notification".into()
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
