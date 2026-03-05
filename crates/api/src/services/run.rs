use std::sync::Arc;

use common::models::{RunRequest, RunRequestMessage, State};
use common::validators::EngineRequestValidator;

use super::ServiceResult;
use crate::infrastructure::{NatsPublisher, RedisClient};
use crate::repositories::{PaginatedResult, Pagination, Run, RunFilter, RunId, RunRepository};
use crate::services::ServiceError;

#[derive(Clone)]
pub struct RunService {
    repo: Arc<dyn RunRepository>,
    nats: Arc<NatsPublisher>,
    redis: Arc<RedisClient>,
}

impl RunService {
    pub fn with_messaging(
        repo: Arc<dyn RunRepository>,
        nats: Arc<NatsPublisher>,
        redis: Arc<RedisClient>,
    ) -> Self {
        Self { repo, nats, redis }
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

        let engine_config = self
            .redis
            .get_engine_config(&req.workflow_engine, &req.workflow_engine_version)
            .await
            .ok_or_else(|| {
                ServiceError::EngineConfigNotFound(
                    req.workflow_engine.clone(),
                    req.workflow_engine_version.clone(),
                )
            })?;

        let validator = EngineRequestValidator::new(engine_config);
        let validated = validator.validate(req).map_err(ServiceError::Validation)?;

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

        self.nats.publish_run(&topic, &message).await.map_err(ServiceError::Messaging)?;

        Ok(())
    }

    pub async fn request_cancel(&self, id: &RunId) -> ServiceResult<Option<String>> {
        loop {
            let run = self
                .repo
                .find_by_id(id)
                .await?
                .ok_or_else(|| ServiceError::RunNotFound(id.to_string()))?;

            match run.state {
                State::Queued => {
                    let updated =
                        self.repo.update_state_if(id, State::Queued, State::Canceled).await?;
                    if updated {
                        return Ok(None);
                    }
                },
                State::Initializing | State::Running => {
                    let engine_id =
                        self.redis.get_assigned_engine(id.as_str()).await.ok_or_else(|| {
                            ServiceError::Messaging(format!("No engine found for run {}", id))
                        })?;

                    self.repo.update_state(id, State::Canceling).await?;

                    self.nats
                        .publish_cancel(&engine_id, id.as_str())
                        .await
                        .map_err(ServiceError::Messaging)?;

                    return Ok(Some(engine_id));
                },
                _ => {
                    return Err(ServiceError::InvalidState(format!(
                        "Cannot cancel run in state: {}",
                        run.state
                    )));
                },
            }
        }
    }

    pub async fn find_active_runs(&self) -> ServiceResult<Vec<Run>> {
        self.repo.find_active_runs().await.map_err(Into::into)
    }

    pub async fn finalize_orphaned_run(&self, id: &RunId, state: State) -> ServiceResult<()> {
        self.repo.finalize_orphaned_run(id, state).await.map_err(Into::into)
    }

    pub async fn delete_run(&self, id: &RunId) -> ServiceResult<()> {
        let run = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::RunNotFound(id.to_string()))?;

        match run.state {
            State::Complete
            | State::ExecutorError
            | State::SystemError
            | State::Canceled
            | State::Preempted => {
                let deleted = self.repo.soft_delete(id).await?;
                if deleted {
                    tracing::info!(
                        run_id = %id,
                        user_id = %run.user_id,
                        state = %run.state,
                        "Run soft deleted"
                    );
                }
                Ok(())
            },
            _ => Err(ServiceError::InvalidState(format!(
                "Cannot delete run in non-terminal state: {}",
                run.state
            ))),
        }
    }

    pub fn redis(&self) -> &Arc<RedisClient> {
        &self.redis
    }
}
