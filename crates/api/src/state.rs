use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::infrastructure::{Database, NatsPublisher, RedisClient};
use crate::repositories::{
    RepositoryError, SqlxLogRepository, SqlxRunRepository, SqlxTaskRepository,
};
use crate::services::{LogService, RunService, TaskService};

#[derive(Clone)]
pub struct Services {
    pub runs: RunService,
    pub tasks: TaskService,
    pub logs: LogService,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub services: Services,
    pub config: Config,
    pub boot_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self, RepositoryError> {
        let db_url = config.database_url.as_deref().unwrap_or("");
        let db = Database::connect(db_url).await?;
        let pool = db.pool().clone();

        let run_repo = Arc::new(SqlxRunRepository::new(pool.clone()));
        let task_repo = Arc::new(SqlxTaskRepository::new(pool.clone()));
        let log_repo = Arc::new(SqlxLogRepository::new(pool));

        let run_service = match &config.nats_url {
            Some(nats_url) if !nats_url.is_empty() => match NatsPublisher::new(nats_url).await {
                Ok(nats) => match &config.redis_url {
                    Some(redis_url) if !redis_url.is_empty() => {
                        match RedisClient::new(redis_url).await {
                            Ok(redis) => RunService::with_messaging(
                                run_repo,
                                Arc::new(nats),
                                Arc::new(redis),
                            ),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to connect to Redis, messaging disabled");
                                RunService::new(run_repo)
                            },
                        }
                    },
                    _ => {
                        tracing::warn!("REDIS_URL not set, messaging disabled");
                        RunService::new(run_repo)
                    },
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to connect to NATS, messaging disabled");
                    RunService::new(run_repo)
                },
            },
            _ => RunService::new(run_repo),
        };

        let services = Services {
            runs: run_service,
            tasks: TaskService::new(task_repo),
            logs: LogService::new(log_repo),
        };

        Ok(Self {
            db,
            services,
            config: config.clone(),
            boot_time: Arc::new(RwLock::new(None)),
        })
    }
}
