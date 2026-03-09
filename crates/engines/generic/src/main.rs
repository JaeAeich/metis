use std::collections::HashMap;

use async_trait::async_trait;
use common::models::{TaskLog, WorkflowFileInfo};
use engine::{Engine, EngineError, EngineResult, server};

struct GenericEngine;

#[async_trait]
impl Engine for GenericEngine {
    fn new() -> Self {
        Self
    }

    async fn get_workflow_results(
        &self,
    ) -> Result<Option<HashMap<String, HashMap<String, WorkflowFileInfo>>>, EngineError> {
        Ok(None)
    }

    async fn get_task_logs(&self) -> Result<Option<Vec<TaskLog>>, EngineError> {
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> EngineResult<()> {
    let engine = GenericEngine;
    server::bootstrap(engine, "metis-engine-generic").await
}
