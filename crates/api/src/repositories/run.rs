use std::collections::HashMap;

use chrono::{DateTime, Utc};
use common::models::RunRequest;
use sqlx::{PgPool, QueryBuilder};

use super::{RepositoryResult, Run, RunId, State, calculate_next_token, pagination_offset};

pub struct SqlxRunRepository {
    pool: PgPool,
}

impl SqlxRunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: &RunId) -> RepositoryResult<Option<Run>> {
        let row = sqlx::query!(
            "SELECT run_id, user_id, state, workflow_type, workflow_type_version, \
            workflow_url, workflow_engine, workflow_engine_version, \
            workflow_params, workflow_engine_parameters, tags, \
            start_time, end_time, created_at, deleted_at FROM runs \
            WHERE run_id = $1 AND deleted_at IS NULL",
            id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Run {
            id: RunId::new(r.run_id),
            user_id: r.user_id,
            state: r.state.parse().unwrap_or(State::Unknown),
            workflow_type: r.workflow_type,
            workflow_type_version: r.workflow_type_version,
            workflow_url: r.workflow_url,
            workflow_engine: r.workflow_engine,
            workflow_engine_version: r.workflow_engine_version,
            workflow_params: r.workflow_params,
            workflow_engine_parameters: r.workflow_engine_parameters,
            tags: r.tags,
            start_time: r.start_time,
            end_time: r.end_time,
            created_at: r.created_at,
            deleted_at: r.deleted_at,
        }))
    }

    pub async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Run>> {
        let offset = pagination_offset(&pagination);
        let limit = pagination.page_size as i64 + 1;

        let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
            "SELECT run_id, user_id, state, workflow_type, workflow_type_version, \
            workflow_url, workflow_engine, workflow_engine_version, \
            workflow_params, workflow_engine_parameters, tags, \
            start_time, end_time, created_at, deleted_at FROM runs WHERE deleted_at IS NULL",
        );

        if let Some(ref state) = filter.state {
            qb.push(" AND state = ").push_bind(state.to_string());
        }
        if let Some(ref user_id) = filter.user_id {
            qb.push(" AND user_id = ").push_bind(user_id.as_str());
        }
        if let Some(ref started_after) = filter.started_after {
            qb.push(" AND start_time >= ").push_bind(*started_after);
        }
        if let Some(ref started_before) = filter.started_before {
            qb.push(" AND start_time <= ").push_bind(*started_before);
        }
        match (&filter.tag_key, &filter.tag_value) {
            (Some(key), Some(value)) => {
                qb.push(" AND tags->>")
                    .push_bind(key.as_str())
                    .push(" = ")
                    .push_bind(value.as_str());
            },
            (Some(key), None) => {
                qb.push(" AND tags ? ").push_bind(key.as_str());
            },
            _ => {},
        }

        qb.push(" ORDER BY created_at DESC LIMIT ").push_bind(limit);
        qb.push(" OFFSET ").push_bind(offset as i64);

        let rows = qb.build().fetch_all(&self.pool).await?;

        use sqlx::Row;
        let runs: Vec<Run> = rows
            .iter()
            .map(|r| Run {
                id: RunId::new(r.get::<String, _>("run_id")),
                user_id: r.get::<String, _>("user_id"),
                state: r.get::<String, _>("state").parse().unwrap_or(State::Unknown),
                workflow_type: r.get("workflow_type"),
                workflow_type_version: r.get("workflow_type_version"),
                workflow_url: r.get("workflow_url"),
                workflow_engine: r.get("workflow_engine"),
                workflow_engine_version: r.get("workflow_engine_version"),
                workflow_params: r.get("workflow_params"),
                workflow_engine_parameters: r.get("workflow_engine_parameters"),
                tags: r
                    .get::<Option<serde_json::Value>, _>("tags")
                    .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
                start_time: r.get("start_time"),
                end_time: r.get("end_time"),
                created_at: r.get("created_at"),
                deleted_at: r.get("deleted_at"),
            })
            .collect();

        Ok(calculate_next_token(runs, pagination.page_size, offset))
    }

    pub async fn count_by_state(&self) -> RepositoryResult<HashMap<String, i64>> {
        let rows = sqlx::query!(
            "SELECT state, COUNT(*) as count FROM runs WHERE deleted_at IS NULL GROUP BY state"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.state, r.count.unwrap_or(0))).collect())
    }

    pub async fn insert_run(
        &self,
        run_id: &str,
        user_id: &str,
        req: &RunRequest,
    ) -> RepositoryResult<()> {
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            ON CONFLICT (run_id) DO NOTHING
            "#,
            run_id,
            user_id,
            State::Queued.to_string(),
            req.workflow_type,
            req.workflow_type_version,
            req.workflow_url,
            req.workflow_engine,
            req.workflow_engine_version,
            workflow_params,
            engine_params,
            tags,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_state(&self, id: &RunId, state: State) -> RepositoryResult<()> {
        sqlx::query!(
            "UPDATE runs SET state = $1 WHERE run_id = $2",
            state.to_string(),
            id.as_str()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_state_if(
        &self,
        id: &RunId,
        expected: State,
        new: State,
    ) -> RepositoryResult<bool> {
        let result = sqlx::query!(
            "UPDATE runs SET state = $1 WHERE run_id = $2 AND state = $3",
            new.to_string(),
            id.as_str(),
            expected.to_string()
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn find_active_runs(&self) -> RepositoryResult<Vec<Run>> {
        let rows = sqlx::query!(
            r#"
            SELECT run_id, user_id, state, workflow_type, workflow_type_version,
                workflow_url, workflow_engine, workflow_engine_version,
                workflow_params, workflow_engine_parameters, tags,
                start_time, end_time, created_at, deleted_at
            FROM runs
            WHERE state IN ('RUNNING', 'INITIALIZING') AND deleted_at IS NULL
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Run {
                id: RunId::new(r.run_id),
                user_id: r.user_id,
                state: r.state.parse().unwrap_or(State::Unknown),
                workflow_type: r.workflow_type,
                workflow_type_version: r.workflow_type_version,
                workflow_url: r.workflow_url,
                workflow_engine: r.workflow_engine,
                workflow_engine_version: r.workflow_engine_version,
                workflow_params: r.workflow_params,
                workflow_engine_parameters: r.workflow_engine_parameters,
                tags: r.tags,
                start_time: r.start_time,
                end_time: r.end_time,
                created_at: r.created_at,
                deleted_at: r.deleted_at,
            })
            .collect())
    }

    pub async fn finalize_orphaned_run(&self, id: &RunId, state: State) -> RepositoryResult<()> {
        sqlx::query!(
            "UPDATE runs SET state = $1, end_time = NOW() WHERE run_id = $2",
            state.to_string(),
            id.as_str()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(&self, id: &RunId) -> RepositoryResult<bool> {
        let result = sqlx::query!(
            "UPDATE runs SET deleted_at = NOW() WHERE run_id = $1 AND deleted_at IS NULL",
            id.as_str()
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn find_run_log(
        &self,
        run_id: &RunId,
    ) -> RepositoryResult<Option<common::models::Log>> {
        let row = sqlx::query!(
            "SELECT name, cmd, start_time::text as start_time, end_time::text as end_time, \
            stdout, stderr, exit_code, system_logs \
            FROM run_logs WHERE run_id = $1 ORDER BY id DESC LIMIT 1",
            run_id.as_str()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| common::models::Log {
            name: r.name,
            cmd: r.cmd.and_then(|v| serde_json::from_value(v).ok()),
            start_time: r.start_time,
            end_time: r.end_time,
            stdout: r.stdout,
            stderr: r.stderr,
            exit_code: r.exit_code,
            system_logs: r.system_logs.and_then(|v| serde_json::from_value(v).ok()),
        }))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    pub state: Option<State>,
    pub user_id: Option<String>,
    pub tag_key: Option<String>,
    pub tag_value: Option<String>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
}

impl RunFilter {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub page_size: u32,
    pub page_token: Option<String>,
}

impl Pagination {
    pub fn new(page_size: u32, page_token: Option<String>) -> Self {
        Self { page_size, page_token }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page_size: 50, page_token: None }
    }
}

#[derive(Debug)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub next_page_token: Option<String>,
}
