use std::sync::Arc;

use common::configs::EngineConfig;
use common::keys;
use redis::aio::MultiplexedConnection;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RedisClient {
    conn: Arc<Mutex<MultiplexedConnection>>,
}

impl RedisClient {
    pub async fn new(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn get_assigned_engine(&self, run_id: &str) -> Option<String> {
        let key = keys::valkey_run_engine(run_id);
        let mut conn = self.conn.lock().await;
        let engine_id: Result<String, _> =
            redis::cmd("GET").arg(&key).query_async(&mut *conn).await;
        engine_id.ok()
    }

    pub async fn get_engine_config(&self, name: &str, version: &str) -> Option<EngineConfig> {
        let key = keys::valkey_engine_config(name, version);
        let mut conn = self.conn.lock().await;
        let config_json: Result<String, _> =
            redis::cmd("GET").arg(&key).query_async(&mut *conn).await;
        config_json.ok().and_then(|json| serde_json::from_str(&json).ok())
    }

    pub async fn list_engine_configs(&self) -> Vec<EngineConfig> {
        // Phase 1: collect all keys via SCAN
        let keys = self.scan_keys(common::keys::valkey_engine_config_pattern()).await;
        if keys.is_empty() {
            return vec![];
        }

        // Phase 2: MGET all keys in one round-trip
        let mut conn = self.conn.lock().await;
        let values: Vec<Option<String>> =
            redis::cmd("MGET").arg(&keys).query_async(&mut *conn).await.unwrap_or_default();
        drop(conn);

        values
            .into_iter()
            .flatten()
            .filter_map(|json| serde_json::from_str::<EngineConfig>(&json).ok())
            .collect()
    }

    /// SCAN all keys matching a pattern, releasing the lock between iterations.
    async fn scan_keys(&self, pattern: &str) -> Vec<String> {
        let mut all_keys = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let result: Result<(u64, Vec<String>), _> = {
                let mut conn = self.conn.lock().await;
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .query_async(&mut *conn)
                    .await
            };

            match result {
                Ok((new_cursor, keys)) => {
                    all_keys.extend(keys);
                    cursor = new_cursor;
                    if cursor == 0 {
                        break;
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, pattern, "Failed to scan keys");
                    break;
                },
            }
        }

        all_keys
    }
}
