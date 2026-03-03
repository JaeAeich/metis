use std::sync::Arc;

use super::ServiceResult;
use crate::repositories::{PaginatedResult, Pagination, RunId, Task, TaskId, TaskRepository};

#[derive(Clone)]
pub struct TaskService {
    repo: Arc<dyn TaskRepository>,
}

impl TaskService {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn find_by_id(
        &self,
        run_id: &RunId,
        task_id: &TaskId,
    ) -> ServiceResult<Option<Task>> {
        self.repo.find_by_id(run_id, task_id).await.map_err(Into::into)
    }

    pub async fn find_by_run(
        &self,
        run_id: &RunId,
        pagination: Pagination,
    ) -> ServiceResult<PaginatedResult<Task>> {
        self.repo.find_by_run(run_id, pagination).await.map_err(Into::into)
    }
}
