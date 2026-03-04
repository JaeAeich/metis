use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub database_url: Option<String>,

    #[serde(default)]
    pub nats_url: Option<String>,
    #[serde(default = "default_notification_subject")]
    pub nats_notification_subject: String,

    #[serde(default)]
    pub redis_url: Option<String>,
}

fn default_notification_subject() -> String {
    "metis.notification".into()
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
