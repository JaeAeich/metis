use std::collections::HashMap;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct BuildContext {
    pub workflow_url: String,
    pub workflow_path: String,
    pub workflow_params: String,
    pub engine_params: String,
    pub workdir: String,
    pub run_id: String,
    pub user_id: String,
    pub params_file: String,
    pub log_dir: String,
    pub output_dir: String,
    pub subdirs: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub date: String,
    pub time: String,
}

#[derive(Debug)]
pub struct CommandInfo {
    pub command: String,
    pub redacted_command: String,
    pub env_vars: HashMap<String, String>,
    pub workdir: String,
}
