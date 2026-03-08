use chrono::{DateTime, Utc};
use common::models::{Log, State, TaskLog, ValidatedRunRequest};
use sqlx::PgPool;
use tracing::debug;

use crate::error::{EngineError, EngineResult};

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

impl Stream {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stream::Stdout => "stdout",
            Stream::Stderr => "stderr",
        }
    }
}

impl Db {
    pub async fn new(database_url: &str) -> EngineResult<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to connect to database: {}", e)))?;
        let display_url = database_url.split('@').next_back().unwrap_or("database");
        debug!(url = %display_url, "Connected to database");
        Ok(Self { pool })
    }

    pub async fn insert_run(
        &self,
        run_id: &str,
        user_id: &str,
        req: &ValidatedRunRequest,
        started_at: DateTime<Utc>,
    ) -> EngineResult<()> {
        let workflow_params = req.workflow_params.clone();
        let engine_params = req
            .workflow_engine_parameters
            .as_ref()
            .map(|params| serde_json::to_value(params).unwrap_or(serde_json::Value::Null));
        let tags = serde_json::to_value(req.tags.as_ref().unwrap_or(&Default::default()))
            .unwrap_or(serde_json::Value::Object(Default::default()));

        sqlx::query!(
            r#"
            INSERT INTO runs (
                run_id, user_id, state,
                workflow_type, workflow_type_version, workflow_url,
                workflow_engine, workflow_engine_version,
                workflow_params, workflow_engine_parameters,
                tags, start_time
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (run_id) DO UPDATE SET state = EXCLUDED.state
            "#,
            run_id,
            user_id,
            State::Initializing as State,
            req.workflow_type,
            req.workflow_type_version,
            req.workflow_url,
            req.workflow_engine,
            req.workflow_engine_version,
            workflow_params,
            engine_params,
            tags,
            started_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EngineError::Generic(format!("Failed to insert run: {}", e)))?;

        Ok(())
    }

    pub async fn get_run_state(&self, run_id: &str) -> EngineResult<Option<State>> {
        let row = sqlx::query!("SELECT state FROM runs WHERE run_id = $1", run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to fetch run state: {}", e)))?;

        Ok(row.map(|r| r.state.parse().unwrap_or(State::Unknown)))
    }

    pub async fn update_run_state(&self, run_id: &str, state: State) -> EngineResult<()> {
        sqlx::query!("UPDATE runs SET state = $1 WHERE run_id = $2", state as State, run_id,)
            .execute(&self.pool)
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to update run state: {}", e)))?;

        Ok(())
    }

    pub async fn finalize_run(
        &self,
        run_id: &str,
        state: State,
        end_time: DateTime<Utc>,
    ) -> EngineResult<()> {
        sqlx::query!(
            "UPDATE runs SET state = $1, end_time = $2 WHERE run_id = $3",
            state as State,
            end_time,
            run_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EngineError::Generic(format!("Failed to finalize run: {}", e)))?;

        Ok(())
    }

    pub async fn insert_run_log(&self, run_id: &str, log: &Log) -> EngineResult<()> {
        let cmd = log
            .cmd
            .as_ref()
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));
        let system_logs = log
            .system_logs
            .as_ref()
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));
        let start_time = log.start_time.as_deref().and_then(|s| s.parse::<DateTime<Utc>>().ok());
        let end_time = log.end_time.as_deref().and_then(|s| s.parse::<DateTime<Utc>>().ok());

        sqlx::query!(
            r#"
            INSERT INTO run_logs (run_id, name, cmd, start_time, end_time, stdout, stderr, exit_code, system_logs)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            run_id,
            log.name,
            cmd,
            start_time,
            end_time,
            log.stdout,
            log.stderr,
            log.exit_code,
            system_logs,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EngineError::Generic(format!("Failed to insert run log: {}", e)))?;

        Ok(())
    }

    pub async fn insert_log_line(
        &self,
        run_id: &str,
        stream: Stream,
        seq: u64,
        line: &str,
    ) -> EngineResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO log_lines (run_id, stream, seq, line)
            VALUES ($1, $2, $3, $4)
            "#,
            run_id,
            stream.as_str(),
            seq as i64,
            line,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EngineError::Generic(format!("Failed to insert log line: {}", e)))?;

        Ok(())
    }

    pub async fn insert_task_logs(&self, run_id: &str, tasks: &[TaskLog]) -> EngineResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to begin transaction: {}", e)))?;

        for task in tasks {
            let cmd = task
                .cmd
                .as_ref()
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));
            let system_logs = task
                .system_logs
                .as_ref()
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));
            let start_time =
                task.start_time.as_deref().and_then(|s| s.parse::<DateTime<Utc>>().ok());
            let end_time = task.end_time.as_deref().and_then(|s| s.parse::<DateTime<Utc>>().ok());

            sqlx::query!(
                r#"
                INSERT INTO task_logs (run_id, task_id, name, cmd, start_time, end_time, stdout, stderr, exit_code, system_logs, tes_uri)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
                run_id,
                task.id,
                task.name,
                cmd,
                start_time,
                end_time,
                task.stdout,
                task.stderr,
                task.exit_code,
                system_logs,
                task.tes_uri,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to insert task log: {}", e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| EngineError::Generic(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    pub async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }
}
