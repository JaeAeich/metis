use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::infrastructure::Database;
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
        let db = Database::connect(&config.database_url).await?;
        let pool = db.pool().clone();

        let run_repo = Arc::new(SqlxRunRepository::new(pool.clone()));
        let task_repo = Arc::new(SqlxTaskRepository::new(pool.clone()));
        let log_repo = Arc::new(SqlxLogRepository::new(pool));

        let services = Services {
            runs: RunService::new(run_repo),
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
