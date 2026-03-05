use std::sync::Arc;

use common::models::{EngineInfo, ServiceInfo, WorkflowEngineVersion, WorkflowTypeVersion};

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

        let engines = configs
            .into_iter()
            .map(|c| EngineInfo {
                name: c.name,
                version: c.version,
                id: c.id.to_string(),
                workflow_types: c.workflow_types,
                workflow_type_versions: c.workflow_type_versions,
                backend: c.backend,
                workflow_params: c.workflow_params,
                engine_params: c.engine_params,
                denied_params: c.denied_params,
                ignored_params: c.ignored_params,
            })
            .collect();

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
                ("environment".to_string(), self.config.environment.clone()),
                ("service".to_string(), self.config.service_name.clone()),
            ]),
            engines,
        })
    }
}
