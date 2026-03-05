use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common::models::RunRequest;
use sqlx::{PgPool, Row};

use super::{RepositoryResult, Run, RunId, State, calculate_next_token, pagination_offset};

#[async_trait]
pub trait RunRepository: Send + Sync {
    async fn find_by_id(&self, id: &RunId) -> RepositoryResult<Option<Run>>;
    async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Run>>;
    async fn count_by_state(&self) -> RepositoryResult<HashMap<String, i64>>;
    async fn insert_run(
        &self,
        run_id: &str,
        user_id: &str,
        req: &RunRequest,
    ) -> RepositoryResult<()>;
    async fn update_state(&self, id: &RunId, state: State) -> RepositoryResult<()>;
    async fn update_state_if(
        &self,
        id: &RunId,
        expected: State,
        new: State,
    ) -> RepositoryResult<bool>;
    async fn find_active_runs(&self) -> RepositoryResult<Vec<Run>>;
    async fn finalize_orphaned_run(&self, id: &RunId, state: State) -> RepositoryResult<()>;
    async fn soft_delete(&self, id: &RunId) -> RepositoryResult<bool>;
}

pub struct SqlxRunRepository {
    pool: PgPool,
}

impl SqlxRunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RunRepository for SqlxRunRepository {
    async fn find_by_id(&self, id: &RunId) -> RepositoryResult<Option<Run>> {
        let row = sqlx::query(
            r#"
            SELECT
                run_id, user_id, state, workflow_type, workflow_type_version,
                workflow_url, workflow_engine, workflow_engine_version,
                workflow_params, workflow_engine_parameters, tags,
                start_time, end_time, created_at, deleted_at
            FROM runs
            WHERE run_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_row_to_run))
    }

    async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Run>> {
        let offset = pagination_offset(&pagination);
        let limit = pagination.page_size as i64;

        let mut query_str = String::from(
            "SELECT run_id, user_id, state, workflow_type, workflow_type_version, \
            workflow_url, workflow_engine, workflow_engine_version, \
            workflow_params, workflow_engine_parameters, tags, \
            start_time, end_time, created_at, deleted_at FROM runs WHERE deleted_at IS NULL",
        );

        let mut bind_idx = 1;

        if filter.state.is_some() {
            query_str.push_str(&format!(" AND state = ${}", bind_idx));
            bind_idx += 1;
        }
        if filter.user_id.is_some() {
            query_str.push_str(&format!(" AND user_id = ${}", bind_idx));
            bind_idx += 1;
        }
        if filter.started_after.is_some() {
            query_str.push_str(&format!(" AND start_time >= ${}", bind_idx));
            bind_idx += 1;
        }
        if filter.started_before.is_some() {
            query_str.push_str(&format!(" AND start_time <= ${}", bind_idx));
            bind_idx += 1;
        }
        if filter.tag_key.is_some() && filter.tag_value.is_some() {
            query_str.push_str(&format!(" AND tags->>${} = ${}", bind_idx, bind_idx + 1));
            bind_idx += 2;
        } else if filter.tag_key.is_some() {
            query_str.push_str(&format!(" AND tags ? ${}", bind_idx));
            bind_idx += 1;
        }

        query_str
            .push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET {}", bind_idx, offset));

        let mut query = sqlx::query(&query_str);

        if let Some(ref state) = filter.state {
            query = query.bind(state.to_string());
        }
        if let Some(ref user_id) = filter.user_id {
            query = query.bind(user_id);
        }
        if let Some(ref started_after) = filter.started_after {
            query = query.bind(started_after);
        }
        if let Some(ref started_before) = filter.started_before {
            query = query.bind(started_before);
        }
        if let (Some(tag_key), Some(tag_value)) = (&filter.tag_key, &filter.tag_value) {
            query = query.bind(tag_key).bind(tag_value);
        } else if let Some(tag_key) = &filter.tag_key {
            query = query.bind(tag_key);
        }

        query = query.bind(limit + 1);

        let rows = query.fetch_all(&self.pool).await?;

        let runs: Vec<Run> = rows.into_iter().map(map_row_to_run).collect();

        Ok(calculate_next_token(runs, pagination.page_size, offset))
    }

    async fn count_by_state(&self) -> RepositoryResult<HashMap<String, i64>> {
        let rows = sqlx::query(
            "SELECT state, COUNT(*) as count FROM runs WHERE deleted_at IS NULL GROUP BY state",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>("state"), r.get::<i64, _>("count")))
            .collect())
    }

    async fn insert_run(
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

        sqlx::query(
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
        )
        .bind(run_id)
        .bind(user_id)
        .bind(State::Queued.to_string())
        .bind(&req.workflow_type)
        .bind(&req.workflow_type_version)
        .bind(&req.workflow_url)
        .bind(&req.workflow_engine)
        .bind(&req.workflow_engine_version)
        .bind(workflow_params)
        .bind(engine_params)
        .bind(tags)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_state(&self, id: &RunId, state: State) -> RepositoryResult<()> {
        sqlx::query("UPDATE runs SET state = $1 WHERE run_id = $2")
            .bind(state.to_string())
            .bind(id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_state_if(
        &self,
        id: &RunId,
        expected: State,
        new: State,
    ) -> RepositoryResult<bool> {
        let result = sqlx::query("UPDATE runs SET state = $1 WHERE run_id = $2 AND state = $3")
            .bind(new.to_string())
            .bind(id.as_str())
            .bind(expected.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn find_active_runs(&self) -> RepositoryResult<Vec<Run>> {
        let rows = sqlx::query(
            r#"
            SELECT run_id, user_id, state, workflow_type, workflow_type_version,
                workflow_url, workflow_engine, workflow_engine_version,
                workflow_params, workflow_engine_parameters, tags,
                start_time, end_time, created_at, deleted_at
            FROM runs
            WHERE state IN ('RUNNING', 'INITIALIZING') AND deleted_at IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_row_to_run).collect())
    }

    async fn finalize_orphaned_run(&self, id: &RunId, state: State) -> RepositoryResult<()> {
        sqlx::query("UPDATE runs SET state = $1, end_time = NOW() WHERE run_id = $2")
            .bind(state.to_string())
            .bind(id.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn soft_delete(&self, id: &RunId) -> RepositoryResult<bool> {
        let result = sqlx::query(
            "UPDATE runs SET deleted_at = NOW() WHERE run_id = $1 AND deleted_at IS NULL",
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

fn map_row_to_run(row: sqlx::postgres::PgRow) -> Run {
    Run {
        id: RunId::new(row.get::<String, _>("run_id")),
        user_id: row.get::<String, _>("user_id"),
        state: row.get::<String, _>("state").parse().unwrap_or(State::Unknown),
        workflow_type: row.get("workflow_type"),
        workflow_type_version: row.get("workflow_type_version"),
        workflow_url: row.get("workflow_url"),
        workflow_engine: row.get("workflow_engine"),
        workflow_engine_version: row.get("workflow_engine_version"),
        workflow_params: row.get("workflow_params"),
        workflow_engine_parameters: row.get("workflow_engine_parameters"),
        tags: row
            .get::<Option<serde_json::Value>, _>("tags")
            .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
        start_time: row.get("start_time"),
        end_time: row.get("end_time"),
        created_at: row.get("created_at"),
        deleted_at: row.get("deleted_at"),
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
