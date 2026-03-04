use std::sync::Arc;

use common::models::{RunRequest, RunRequestMessage, State, ValidatedRunRequest};

use super::ServiceResult;
use crate::infrastructure::{NatsPublisher, RedisClient};
use crate::repositories::{PaginatedResult, Pagination, Run, RunFilter, RunId, RunRepository};
use crate::services::ServiceError;

#[derive(Clone)]
pub struct RunService {
    repo: Arc<dyn RunRepository>,
    nats: Option<Arc<NatsPublisher>>,
    redis: Option<Arc<RedisClient>>,
}

impl RunService {
    pub fn new(repo: Arc<dyn RunRepository>) -> Self {
        Self { repo, nats: None, redis: None }
    }

    pub fn with_messaging(
        repo: Arc<dyn RunRepository>,
        nats: Arc<NatsPublisher>,
        redis: Arc<RedisClient>,
    ) -> Self {
        Self { repo, nats: Some(nats), redis: Some(redis) }
    }

    pub async fn find_by_id(&self, id: &RunId) -> ServiceResult<Option<Run>> {
        self.repo.find_by_id(id).await.map_err(Into::into)
    }

    pub async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> ServiceResult<PaginatedResult<Run>> {
        self.repo.find_all(filter, pagination).await.map_err(Into::into)
    }

    pub async fn create_run(
        &self,
        run_id: &str,
        user_id: &str,
        req: &RunRequest,
    ) -> ServiceResult<()> {
        self.repo.insert_run(run_id, user_id, req).await?;

        let nats = self
            .nats
            .as_ref()
            .ok_or_else(|| ServiceError::Messaging("NATS not configured".to_string()))?;

        let validated = ValidatedRunRequest {
            workflow_params: req.workflow_params.clone(),
            workflow_type: req.workflow_type.clone(),
            workflow_type_version: req.workflow_type_version.clone(),
            tags: req.tags.clone(),
            workflow_engine_parameters: None,
            workflow_engine: req.workflow_engine.clone(),
            workflow_engine_version: req.workflow_engine_version.clone(),
            workflow_url: req.workflow_url.clone(),
        };

        let message = RunRequestMessage {
            run_id: run_id.to_string(),
            request: validated,
            user_id: user_id.to_string(),
        };

        let topic = format!(
            "metis.runs.{}.{}.{}.{}",
            req.workflow_engine,
            req.workflow_engine_version,
            req.workflow_type,
            req.workflow_type_version
        );

        nats.publish_run(&topic, &message).await.map_err(ServiceError::Messaging)?;

        Ok(())
    }

    pub async fn request_cancel(&self, id: &RunId) -> ServiceResult<Option<String>> {
        let run = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::RunNotFound(id.to_string()))?;

        match run.state {
            State::Queued => {
                self.repo.update_state(id, State::Canceled).await?;
                return Ok(None);
            },
            State::Initializing | State::Running => {},
            _ => {
                return Err(ServiceError::InvalidState(format!(
                    "Cannot cancel run in state: {}",
                    run.state
                )));
            },
        }

        let nats = self
            .nats
            .as_ref()
            .ok_or_else(|| ServiceError::Messaging("NATS not configured".to_string()))?;

        let engine_id = if let Some(redis) = &self.redis {
            redis
                .get_assigned_engine(id.as_str())
                .await
                .ok_or_else(|| ServiceError::Messaging(format!("No engine found for run {}", id)))?
        } else {
            return Err(ServiceError::Messaging("Redis not configured".to_string()));
        };

        self.repo.update_state(id, State::Canceling).await?;

        nats.publish_cancel(&engine_id, id.as_str())
            .await
            .map_err(ServiceError::Messaging)?;

        Ok(Some(engine_id))
    }

    pub async fn find_active_runs(&self) -> ServiceResult<Vec<Run>> {
        self.repo.find_active_runs().await.map_err(Into::into)
    }

    pub async fn finalize_orphaned_run(&self, id: &RunId, state: State) -> ServiceResult<()> {
        self.repo.finalize_orphaned_run(id, state).await.map_err(Into::into)
    }

    pub fn redis(&self) -> Option<&Arc<RedisClient>> {
        self.redis.as_ref()
    }
}
