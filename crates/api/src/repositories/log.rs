use sqlx::{PgPool, Row};

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

        let rows = sqlx::query(
            r#"
            SELECT run_id, stream, seq, line, written_at
            FROM log_lines
            WHERE run_id = $1
            AND ($2::text IS NULL OR stream = $2)
            AND ($3::bigint IS NULL OR seq > $3)
            ORDER BY seq ASC
            LIMIT $4
            "#,
        )
        .bind(run_id.as_str())
        .bind(stream.map(|s| s.to_string()))
        .bind(after_seq)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let lines: Vec<LogLine> = rows.into_iter().map(map_row_to_log_line).collect();

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

fn map_row_to_log_line(row: sqlx::postgres::PgRow) -> LogLine {
    LogLine {
        run_id: RunId::new(row.get::<String, _>("run_id")),
        stream: parse_stream(&row.get::<String, _>("stream")),
        seq: row.get("seq"),
        line: row.get("line"),
        written_at: row.get("written_at"),
    }
}

fn parse_stream(s: &str) -> LogStream {
    match s {
        "stdout" => LogStream::Stdout,
        "stderr" => LogStream::Stderr,
        _ => LogStream::Stdout,
    }
}
