use std::sync::Arc;

use common::models::{
    DatabaseStats, ServiceInfo, Stats, SystemInfo, WorkflowEngineVersion, WorkflowTypeVersion,
};
use sysinfo::System;

use super::ServiceResult;
use crate::config::Config;
use crate::infrastructure::RedisClient;
use crate::repositories::RunRepository;

#[derive(Clone)]
pub struct ServiceInfoService {
    repo: Arc<dyn RunRepository>,
    redis: Arc<RedisClient>,
    config: Config,
}

impl ServiceInfoService {
    pub fn new(repo: Arc<dyn RunRepository>, redis: Arc<RedisClient>, config: Config) -> Self {
        Self { repo, redis, config }
    }

    pub async fn get_service_info(&self) -> ServiceResult<ServiceInfo> {
        let state_counts = self.repo.count_by_state().await?;
        let configs = self.redis.list_engine_configs().await;

        let mut workflow_type_versions: std::collections::HashMap<String, WorkflowTypeVersion> =
            std::collections::HashMap::new();
        let mut workflow_engine_versions: std::collections::HashMap<String, WorkflowEngineVersion> =
            std::collections::HashMap::new();

        for config in &configs {
            for wt in &config.workflow_types {
                workflow_type_versions
                    .entry(wt.clone())
                    .or_insert_with(|| WorkflowTypeVersion { workflow_type_version: Some(vec![]) })
                    .workflow_type_version
                    .as_mut()
                    .unwrap()
                    .extend(config.workflow_type_versions.iter().cloned());
            }

            workflow_engine_versions
                .entry(config.name.clone())
                .or_insert_with(|| WorkflowEngineVersion { workflow_engine_version: Some(vec![]) })
                .workflow_engine_version
                .as_mut()
                .unwrap()
                .push(config.version.clone());
        }

        for v in workflow_type_versions.values_mut() {
            if let Some(ref mut versions) = v.workflow_type_version {
                versions.sort();
                versions.dedup();
            }
        }

        for v in workflow_engine_versions.values_mut() {
            if let Some(ref mut versions) = v.workflow_engine_version {
                versions.sort();
                versions.dedup();
            }
        }

        Ok(ServiceInfo {
            workflow_type_versions,
            supported_wes_versions: vec!["1.1.0".to_string()],
            supported_filesystem_protocols: vec![
                "http".to_string(),
                "https".to_string(),
                "s3".to_string(),
                "file".to_string(),
            ],
            workflow_engine_versions,
            default_workflow_engine_parameters: vec![],
            system_state_counts: state_counts,
            auth_instructions_url: self.config.auth_instructions_url.clone(),
            tags: std::collections::HashMap::from([
                ("environment".to_string(), "production".to_string()),
                ("service".to_string(), "metis-api".to_string()),
            ]),
        })
    }

    pub async fn get_stats(&self) -> ServiceResult<Stats> {
        let total_runs = self.repo.count_total().await?;
        let active_runs = self.repo.count_active().await?;
        let engines = self.redis.list_running_engines().await;
        let system = self.get_system_info();

        Ok(Stats {
            system,
            database: DatabaseStats { total_runs, active_runs },
            engines,
        })
    }

    fn get_system_info(&self) -> SystemInfo {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let total_memory = sys.total_memory() / 1024 / 1024;
        let used_memory = sys.used_memory() / 1024 / 1024;

        let uptime = System::uptime();

        let load_avg = System::load_average();
        let load_average = if load_avg.one != 0.0 {
            Some([load_avg.one, load_avg.five, load_avg.fifteen])
        } else {
            None
        };

        SystemInfo {
            cpu_usage_percent: cpu_usage,
            memory_used_mb: used_memory,
            memory_total_mb: total_memory,
            uptime_secs: uptime,
            load_average,
        }
    }
}
