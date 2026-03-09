use sqlx::PgPool;

use super::{
    PaginatedResult, Pagination, RepositoryResult, RunId, Task, TaskId, calculate_next_token,
    pagination_offset,
};

pub struct SqlxTaskRepository {
    pool: PgPool,
}

impl SqlxTaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SqlxTaskRepository {
    pub async fn find_by_id(
        &self,
        run_id: &RunId,
        task_id: &TaskId,
    ) -> RepositoryResult<Option<Task>> {
        let row = sqlx::query!(
            r#"
            SELECT
                task_id, run_id, name, cmd, start_time, end_time,
                stdout, stderr, exit_code, system_logs, tes_uri
            FROM task_logs
            WHERE run_id = $1 AND task_id = $2
            "#,
            run_id.as_str(),
            task_id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Task {
            id: TaskId::new(r.task_id),
            run_id: RunId::new(r.run_id),
            name: r.name,
            cmd: r.cmd.and_then(|v| serde_json::from_value(v).ok()),
            start_time: r.start_time,
            end_time: r.end_time,
            stdout: r.stdout,
            stderr: r.stderr,
            exit_code: r.exit_code,
            system_logs: r.system_logs.and_then(|v| serde_json::from_value(v).ok()),
            tes_uri: r.tes_uri,
        }))
    }

    pub async fn find_by_run(
        &self,
        run_id: &RunId,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Task>> {
        let offset = pagination_offset(&pagination);
        let limit = pagination.page_size as i64;

        let rows = sqlx::query!(
            r#"
            SELECT
                task_id, run_id, name, cmd, start_time, end_time,
                stdout, stderr, exit_code, system_logs, tes_uri
            FROM task_logs
            WHERE run_id = $1
            ORDER BY id
            LIMIT $2 OFFSET $3
            "#,
            run_id.as_str(),
            limit + 1,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let tasks: Vec<Task> = rows
            .into_iter()
            .map(|r| Task {
                id: TaskId::new(r.task_id),
                run_id: RunId::new(r.run_id),
                name: r.name,
                cmd: r.cmd.and_then(|v| serde_json::from_value(v).ok()),
                start_time: r.start_time,
                end_time: r.end_time,
                stdout: r.stdout,
                stderr: r.stderr,
                exit_code: r.exit_code,
                system_logs: r.system_logs.and_then(|v| serde_json::from_value(v).ok()),
                tes_uri: r.tes_uri,
            })
            .collect();

        Ok(calculate_next_token(tasks, pagination.page_size, offset))
    }
}
