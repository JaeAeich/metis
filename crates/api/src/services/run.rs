use std::sync::Arc;

use super::ServiceResult;
use crate::repositories::{PaginatedResult, Pagination, Run, RunFilter, RunId, RunRepository};

#[derive(Clone)]
pub struct RunService {
    repo: Arc<dyn RunRepository>,
}

impl RunService {
    pub fn new(repo: Arc<dyn RunRepository>) -> Self {
        Self { repo }
    }

    pub async fn find_by_id(&self, id: &RunId) -> ServiceResult<Option<Run>> {
        self.repo.find_by_id(id).await.map_err(Into::into)
    }

    pub async fn find_all(
        &self,
        filter: RunFilter,
        pagination: Pagination,
    ) -> ServiceResult<PaginatedResult<Run>> {
        self.repo.find_all(filter, pagination).await.map_err(Into::into)
    }
}
