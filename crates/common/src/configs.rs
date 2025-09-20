use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Base configuration shared across all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    /// Service name for logging and identification
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// Environment (development, staging, production)
    #[serde(default = "default_environment")]
    pub environment: String,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Enable JSON logging format
    #[serde(default)]
    pub json_logging: bool,
    /// Enable console logging
    #[serde(default = "default_true")]
    pub console_logging: bool,
}

/// NATS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    #[serde(default = "default_nats_url")]
    pub url: String,
    #[serde(default = "default_notification_subject")]
    pub notification_subject: String,
}

/// Valkey (Redis-compatible) configuration
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValkeyConfig {
    /// Valkey server URL, e.g., "redis://localhost:6379"
    #[serde(default = "default_valkey_url")]
    pub url: String,

    /// Optional password (if Valkey requires authentication)
    #[serde(default)]
    pub password: Option<String>,

    /// Optional database index (default is 0)
    #[serde(default = "default_valkey_db")]
    pub db: u8,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
}

impl BaseConfig {
    pub fn from_env() -> Self {
        envy::prefixed("METIS_")
            .from_env::<Self>()
            .unwrap_or_default()
    }
}

impl NatsConfig {
    pub fn from_env() -> Self {
        envy::prefixed("NATS_")
            .from_env::<Self>()
            .unwrap_or_default()
    }
}

impl ValkeyConfig {
    pub fn from_env() -> Self {
        envy::prefixed("VALKEY_")
            .from_env::<Self>()
            .unwrap_or_default()
    }
}
impl DatabaseConfig {
    pub fn from_env() -> Self {
        envy::prefixed("DB_").from_env::<Self>().unwrap_or_default()
    }
}

/// Defaults
impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            service_name: "metis".into(),
            environment: "development".into(),
            log_level: "info".into(),
            json_logging: true,
            console_logging: true,
        }
    }
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".into(),
            notification_subject: default_notification_subject(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/metis".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    pub name: String,
    #[serde(default = "default_uuid_v7")]
    pub id: Uuid,
    pub version: String,
    pub workflow_types: Vec<String>,
    pub workflow_type_versions: Vec<String>,
    pub command_template: String,
    pub backend: Backend,
    pub workflow_params: WorkflowParamsConfig,
    pub engine_params: EngineParamsConfig,
    pub denied_params: Vec<DeniedParam>,
    pub ignored_params: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FullEngineConfig {
    pub version: String,
    pub engine: EngineConfig,
    pub runs: RunsConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Local,
    TES,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkflowParamsConfig {
    pub style: WorkflowParamsStyle,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineParamsConfig {
    pub unknown_params_behavior: UnknownBehavior,
    pub validated_params: Vec<EngineParam>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkdirConfig {
    pub base: String,
    pub pattern: String,
    #[serde(default)]
    pub subdirs: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubDirConfig {
    pub logs: Option<String>,
    pub outputs: Option<String>,
    pub word: Option<String>,
    pub temp: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UnknownBehavior {
    Reject,
    Pass,
    Strip,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowParamsStyle {
    pub method: WorkflowParamsMethod,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub key_value_format: Option<String>,
    #[serde(default)]
    pub format: Option<WorkflowParamsFormat>,
    #[serde(default)]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowParamsMethod {
    #[default]
    Inline,
    File,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowParamsFormat {
    #[default]
    Json,
    Yaml,
    Properties,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BooleanStyle {
    Flag,
    Value,
    Equals,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineParam {
    pub names: Vec<String>,
    pub cli_flag: String,
    #[serde(rename = "type")]
    pub param_type: ParamType,
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub strict_default: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub list_separator: Option<String>,
    #[serde(default)]
    pub boolean_style: Option<BooleanStyle>,
    #[serde(default)]
    pub env_var: Option<String>,
    #[serde(default)]
    pub validate: Option<Vec<Validation>>,
    #[serde(default)]
    pub description: Option<String>,
}

impl PartialEq for BooleanStyle {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Int,
    Float,
    Bool,
    List,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Validation {
    Enum {
        allowed: Vec<String>,
        #[serde(default)]
        message: Option<String>,
    },
    Regex {
        pattern: String,
        #[serde(default)]
        message: Option<String>,
    },
    Range {
        #[serde(default)]
        min: Option<i64>,
        #[serde(default)]
        max: Option<i64>,
        #[serde(default)]
        message: Option<String>,
    },
    Custom {
        rule: String,
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(alias = "file_exists")]
    FileExists {
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(alias = "file_extension")]
    FileExtension {
        allowed: Vec<String>,
        #[serde(default)]
        message: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeniedParam {
    pub pattern: String,
    #[serde(rename = "type")]
    pub match_type: MatchType,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    Exact,
    Regex,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunsConfig {
    pub workdir: WorkdirConfig,
    #[serde(default)]
    pub config: Option<RunConfigTemplate>,
    #[serde(default)]
    pub hooks: Option<RunHooks>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RunConfigTemplate {
    pub filepath: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunHooks {
    #[serde(default)]
    pub pre_run_hook: Option<Hook>,
    #[serde(default)]
    pub on_success_hook: Option<Hook>,
    #[serde(default)]
    pub on_failure_hook: Option<Hook>,
    #[serde(default)]
    pub post_run_hook: Option<Hook>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hook {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
}

/// Default helpers
fn default_service_name() -> String {
    "metis".into()
}
fn default_environment() -> String {
    "development".into()
}
fn default_log_level() -> String {
    "info".into()
}
fn default_nats_url() -> String {
    "nats://localhost:4222".into()
}
fn default_notification_subject() -> String {
    default_service_name() + ".notification"
}
fn default_database_url() -> String {
    "postgresql://localhost:5432/metis".into()
}
fn default_true() -> bool {
    true
}

fn default_valkey_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_valkey_db() -> u8 {
    0
}

fn default_uuid_v7() -> Uuid {
    Uuid::now_v7()
}
