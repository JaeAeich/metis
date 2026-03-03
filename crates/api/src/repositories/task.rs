use async_trait::async_trait;
use sqlx::{PgPool, Row};

use super::{
    PaginatedResult, Pagination, RepositoryResult, RunId, Task, TaskId, calculate_next_token,
    pagination_offset,
};

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn find_by_id(&self, run_id: &RunId, task_id: &TaskId) -> RepositoryResult<Option<Task>>;
    async fn find_by_run(
        &self,
        run_id: &RunId,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Task>>;
}

pub struct SqlxTaskRepository {
    pool: PgPool,
}

impl SqlxTaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TaskRepository for SqlxTaskRepository {
    async fn find_by_id(&self, run_id: &RunId, task_id: &TaskId) -> RepositoryResult<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT
                task_id, run_id, name, cmd, start_time, end_time,
                stdout, stderr, exit_code, system_logs, tes_uri
            FROM task_logs
            WHERE run_id = $1 AND task_id = $2
            "#,
        )
        .bind(run_id.as_str())
        .bind(task_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_row_to_task))
    }

    async fn find_by_run(
        &self,
        run_id: &RunId,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Task>> {
        let offset = pagination_offset(&pagination);
        let limit = pagination.page_size as i64;

        let rows = sqlx::query(
            r#"
            SELECT
                task_id, run_id, name, cmd, start_time, end_time,
                stdout, stderr, exit_code, system_logs, tes_uri
            FROM task_logs
            WHERE run_id = $1
            ORDER BY id
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(run_id.as_str())
        .bind(limit + 1)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        let tasks: Vec<Task> = rows.into_iter().map(map_row_to_task).collect();

        Ok(calculate_next_token(tasks, pagination.page_size, offset))
    }
}

fn map_row_to_task(row: sqlx::postgres::PgRow) -> Task {
    Task {
        id: TaskId::new(row.get::<String, _>("task_id")),
        run_id: RunId::new(row.get::<String, _>("run_id")),
        name: row.get::<Option<String>, _>("name").unwrap_or_default(),
        cmd: row
            .get::<Option<serde_json::Value>, _>("cmd")
            .and_then(|v| serde_json::from_value(v).ok()),
        start_time: row.get("start_time"),
        end_time: row.get("end_time"),
        stdout: row.get("stdout"),
        stderr: row.get("stderr"),
        exit_code: row.get("exit_code"),
        system_logs: row
            .get::<Option<serde_json::Value>, _>("system_logs")
            .and_then(|v| serde_json::from_value(v).ok()),
        tes_uri: row.get("tes_uri"),
    }
}
