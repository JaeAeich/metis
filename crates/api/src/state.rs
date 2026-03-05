use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::infrastructure::{Database, NatsPublisher, RedisClient};
use crate::repositories::{
    RepositoryError, SqlxLogRepository, SqlxRunRepository, SqlxTaskRepository,
};
use crate::services::{LogService, RunService, ServiceInfoService, TaskService};

#[derive(Clone)]
pub struct Services {
    pub runs: RunService,
    pub tasks: TaskService,
    pub logs: LogService,
    pub service_info: ServiceInfoService,
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
        let db = Database::connect(&config.database_url).await?;
        let pool = db.pool().clone();

        let run_repo = Arc::new(SqlxRunRepository::new(pool.clone()));
        let task_repo = Arc::new(SqlxTaskRepository::new(pool.clone()));
        let log_repo = Arc::new(SqlxLogRepository::new(pool));

        let nats = NatsPublisher::new(&config.nats_url)
            .await
            .map_err(|e| RepositoryError::Connection(e.to_string()))?;
        let redis = RedisClient::new(&config.redis_url)
            .await
            .map_err(|e| RepositoryError::Connection(e.to_string()))?;

        let run_service =
            RunService::with_messaging(run_repo.clone(), Arc::new(nats), Arc::new(redis.clone()));
        let service_info_service =
            ServiceInfoService::new(run_repo, Arc::new(redis), config.clone());

        let services = Services {
            runs: run_service,
            tasks: TaskService::new(task_repo),
            logs: LogService::new(log_repo),
            service_info: service_info_service,
        };

        Ok(Self {
            db,
            services,
            config: config.clone(),
            boot_time: Arc::new(RwLock::new(None)),
        })
    }
}
