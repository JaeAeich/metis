use std::collections::HashMap;

use chrono::Utc;
use common::configs::FullEngineConfig;
use telemetry::tracing::error;

use crate::error::{EngineError, EngineResult};
use crate::models::BuildContext;

pub struct WorkdirPaths {
    pub workdir: String,
    pub subdirs: HashMap<String, String>,
}

pub struct WorkdirManager;

impl WorkdirManager {
    pub fn create(
        config: &FullEngineConfig,
        run_id: &str,
        user_id: &str,
    ) -> EngineResult<BuildContext> {
        let timestamp = Utc::now();
        let safe_user_id = user_id.replace('/', "_");

        let pattern = config
            .runs
            .workdir
            .pattern
            .replace("{run_id}", run_id)
            .replace("{user_id}", &safe_user_id)
            .replace("{date}", &timestamp.format("%Y-%m-%d").to_string())
            .replace("{time}", &timestamp.format("%H-%M-%S").to_string())
            .replace("{timestamp}", &timestamp.timestamp().to_string());

        let workdir = format!("{}/{}", config.runs.workdir.base, pattern);
        std::fs::create_dir_all(&workdir).map_err(|e| {
            error!("Failed to create workdir '{}': {}", workdir, e);
            EngineError::Io(e)
        })?;

        let mut subdirs = HashMap::new();
        if let Some(ref subdir_config) = config.runs.workdir.subdirs {
            for (name, subdir) in subdir_config {
                let full_path = format!("{}/{}", workdir, subdir);
                std::fs::create_dir_all(&full_path).map_err(|e| {
                    error!("Failed to create subdir '{}': {}", full_path, e);
                    EngineError::Io(e)
                })?;
                subdirs.insert(name.clone(), full_path);
            }
        }

        let ctx = BuildContext {
            run_id: run_id.to_string(),
            user_id: safe_user_id,
            workflow_path: String::new(),
            workflow_url: String::new(),
            workdir,
            subdirs: subdirs.clone(),
            timestamp,
            workflow_params: String::new(),
            engine_params: String::new(),
            params_file: String::new(),
            log_dir: subdirs.get("logs").cloned().unwrap_or_default(),
            output_dir: subdirs.get("outputs").cloned().unwrap_or_default(),
            date: timestamp.format("%Y-%m-%d").to_string(),
            time: timestamp.format("%H-%M-%S").to_string(),
        };

        // Add per run level config
        if let Some(ref run_config) = config.runs.config {
            let filepath = ctx.replace_vars(&run_config.filepath);
            let content = ctx.replace_vars(&run_config.content);
            let full_path = format!("{}/{}", ctx.workdir, filepath);
            std::fs::write(&full_path, &content).map_err(|e| {
                error!("Failed to write config file '{}': {}", full_path, e);
                EngineError::Io(e)
            })?;
        }

        Ok(ctx)
    }

    pub async fn setup(workdir: &str) -> EngineResult<()> {
        tokio::fs::create_dir_all(workdir).await.map_err(|e| {
            error!("Failed to create workdir '{}': {}", workdir, e);
            EngineError::Io(e)
        })?;
        Ok(())
    }
}
