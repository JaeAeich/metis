use std::sync::Arc;

use super::ServiceResult;
use crate::repositories::{LogLine, LogRepository, LogStream, PaginatedResult, RunId};

#[derive(Clone)]
pub struct LogService {
    repo: Arc<dyn LogRepository>,
}

impl LogService {
    pub fn new(repo: Arc<dyn LogRepository>) -> Self {
        Self { repo }
    }

    pub async fn find_lines(
        &self,
        run_id: &RunId,
        stream: Option<LogStream>,
        after_seq: Option<i64>,
        page_size: u32,
    ) -> ServiceResult<PaginatedResult<LogLine>> {
        self.repo
            .find_lines(run_id, stream, after_seq, page_size)
            .await
            .map_err(Into::into)
    }
}
