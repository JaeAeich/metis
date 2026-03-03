use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, Sse};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

use crate::api::ApiResult;
use crate::extractors::LogPageParams;
use crate::repositories::RunId;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogLineResponse {
    pub run_id: String,
    pub stream: String,
    pub seq: i64,
    pub line: String,
    pub written_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogLineListResponse {
    pub lines: Vec<LogLineResponse>,
    pub next_after_seq: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/runs/{run_id}/logs",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID"),
        LogPageParams
    ),
    responses(
        (status = 200, description = "Paginated log lines", body = LogLineListResponse),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_log_lines(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<LogPageParams>,
) -> ApiResult<Json<LogLineListResponse>> {
    let id = RunId::new(run_id);
    let stream = params.stream.as_ref().and_then(|s| s.parse().ok());
    let page_size = params.page_size();

    let result = app.services.logs.find_lines(&id, stream, params.after_seq, page_size).await?;

    let lines: Vec<LogLineResponse> = result
        .items
        .into_iter()
        .map(|l| LogLineResponse {
            run_id: l.run_id.into_inner(),
            stream: l.stream.to_string(),
            seq: l.seq,
            line: l.line,
            written_at: l.written_at.to_rfc3339(),
        })
        .collect();

    let next_after_seq = lines.last().map(|l| l.seq);

    Ok(Json(LogLineListResponse { lines, next_after_seq }))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StreamParams {
    pub stream: Option<String>,
    pub after_seq: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/runs/{run_id}/logs/stream",
    tag = "Runs",
    params(
        ("run_id" = String, Path, description = "Workflow run ID"),
        StreamParams
    ),
    responses(
        (status = 200, description = "SSE stream of log lines", content_type = "text/event-stream"),
        (status = 404, description = "Run not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn stream_log_lines(
    State(app): State<AppState>,
    Path(run_id): Path<String>,
    Query(params): Query<StreamParams>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>> {
    let id = RunId::new(run_id.clone());
    let stream_type = params.stream.as_ref().and_then(|s| s.parse().ok());

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(100);

    tokio::spawn(async move {
        let mut current_seq = params.after_seq.unwrap_or(0);

        loop {
            match app.services.logs.find_lines(&id, stream_type, Some(current_seq), 50).await {
                Ok(result) => {
                    if result.items.is_empty() {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }

                    for line in result.items {
                        current_seq = line.seq;

                        let log_response = LogLineResponse {
                            run_id: line.run_id.into_inner(),
                            stream: line.stream.to_string(),
                            seq: line.seq,
                            line: line.line,
                            written_at: line.written_at.to_rfc3339(),
                        };

                        let json = serde_json::to_string(&log_response).unwrap_or_default();
                        let event = Event::default().data(json);

                        if tx.send(Ok(event)).await.is_err() {
                            return;
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Error fetching log lines: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                },
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}
