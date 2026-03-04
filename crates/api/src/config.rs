use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub app_port: u16,

    #[serde(default)]
    pub database_url: Option<String>,

    #[serde(default = "default_auth_url")]
    pub auth_instructions_url: String,

    #[serde(default)]
    pub nats_url: Option<String>,

    #[serde(default)]
    pub redis_url: Option<String>,
}

fn default_port() -> u16 {
    8080
}

fn default_auth_url() -> String {
    "https://example.com/auth".to_string()
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
