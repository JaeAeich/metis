use std::collections::HashMap;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use super::{RepositoryResult, Run, RunId, State};

#[async_trait]
pub trait RunRepository: Send + Sync {
    async fn find_by_id(&self, id: &RunId) -> RepositoryResult<Option<Run>>;
    async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> RepositoryResult<PaginatedResult<Run>>;
    async fn count_by_state(&self) -> RepositoryResult<HashMap<String, i64>>;
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
                start_time, end_time, created_at
            FROM runs
            WHERE run_id = $1
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
        let offset = decode_offset(&pagination.page_token).unwrap_or(0);
        let limit = pagination.page_size as i64;

        let mut query_str = String::from(
            "SELECT run_id, user_id, state, workflow_type, workflow_type_version, \
            workflow_url, workflow_engine, workflow_engine_version, \
            workflow_params, workflow_engine_parameters, tags, \
            start_time, end_time, created_at FROM runs WHERE 1=1",
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

        let has_more = runs.len() > pagination.page_size as usize;
        let items: Vec<Run> = runs.into_iter().take(pagination.page_size as usize).collect();

        let next_page_token = if has_more {
            Some(encode_offset(offset + pagination.page_size as u64))
        } else {
            None
        };

        Ok(PaginatedResult { items, next_page_token })
    }

    async fn count_by_state(&self) -> RepositoryResult<HashMap<String, i64>> {
        let rows = sqlx::query("SELECT state, COUNT(*) as count FROM runs GROUP BY state")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>("state"), r.get::<i64, _>("count")))
            .collect())
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
    }
}

fn decode_offset(token: &Option<String>) -> Option<u64> {
    token.as_ref().and_then(|t| {
        let decoded = base64::engine::general_purpose::STANDARD.decode(t).ok()?;
        String::from_utf8(decoded).ok()?.parse().ok()
    })
}

fn encode_offset(offset: u64) -> String {
    base64::engine::general_purpose::STANDARD.encode(offset.to_string())
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
