use sqlx::PgPool;

use super::{LogLine, LogStream, PaginatedResult, RepositoryResult, RunId};

pub struct SqlxLogRepository {
    pool: PgPool,
}

impl SqlxLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SqlxLogRepository {
    pub async fn find_lines(
        &self,
        run_id: &RunId,
        stream: Option<LogStream>,
        after_seq: Option<i64>,
        page_size: u32,
    ) -> RepositoryResult<PaginatedResult<LogLine>> {
        let limit = page_size as i64 + 1;
        let stream_str = stream.map(|s| s.to_string());

        let rows = sqlx::query!(
            r#"
            SELECT run_id, stream, seq, line, written_at
            FROM log_lines
            WHERE run_id = $1
            AND ($2::text IS NULL OR stream = $2)
            AND ($3::bigint IS NULL OR seq > $3)
            ORDER BY seq ASC
            LIMIT $4
            "#,
            run_id.as_str(),
            stream_str,
            after_seq,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        let lines: Vec<LogLine> = rows
            .into_iter()
            .map(|r| LogLine {
                run_id: RunId::new(r.run_id),
                stream: r.stream.parse().unwrap_or(LogStream::Stdout),
                seq: r.seq,
                line: r.line,
                written_at: r.written_at,
            })
            .collect();

        let has_more = lines.len() > page_size as usize;
        let items: Vec<LogLine> = lines.into_iter().take(page_size as usize).collect();

        let next_page_token = if has_more {
            items.last().map(|l| l.seq.to_string())
        } else {
            None
        };

        Ok(PaginatedResult { items, next_page_token })
    }
}
