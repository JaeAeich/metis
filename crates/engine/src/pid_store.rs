use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::clients::Valkey;
use crate::error::EngineResult;

struct EngineProcessHandle {
    pid: u32,
}

pub struct PidStore {
    local: RwLock<HashMap<Uuid, EngineProcessHandle>>,
    valkey: Option<Arc<Valkey>>,
}

impl PidStore {
    pub fn new(valkey: Option<Arc<Valkey>>) -> Self {
        Self { local: RwLock::new(HashMap::new()), valkey }
    }

    pub async fn store(&self, run_id: &str, pid: u32) -> EngineResult<()> {
        if let Ok(run_uuid) = Uuid::parse_str(run_id) {
            self.local.write().await.insert(run_uuid, EngineProcessHandle { pid });
        }

        if let Some(valkey) = &self.valkey {
            valkey.store_run_pid(run_id, pid).await?;
        }

        Ok(())
    }

    pub async fn get(&self, run_id: &str) -> EngineResult<Option<u32>> {
        if let Ok(run_uuid) = Uuid::parse_str(run_id)
            && let Some(handle) = self.local.read().await.get(&run_uuid)
        {
            return Ok(Some(handle.pid));
        }

        if let Some(valkey) = &self.valkey {
            return valkey.get_run_pid(run_id).await;
        }

        Ok(None)
    }

    pub async fn remove(&self, run_id: &str) -> EngineResult<()> {
        if let Ok(run_uuid) = Uuid::parse_str(run_id) {
            self.local.write().await.remove(&run_uuid);
        }

        if let Some(valkey) = &self.valkey {
            valkey.remove_run_pid(run_id).await?;
        }

        Ok(())
    }
}
