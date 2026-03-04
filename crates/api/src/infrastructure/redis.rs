use std::sync::Arc;

use common::configs::EngineConfig;
use redis::aio::MultiplexedConnection;
use tokio::sync::Mutex;

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
        let key = format!("metis.runs.{}.engine", run_id);
        let mut conn = self.conn.lock().await;
        let engine_id: Result<String, _> =
            redis::cmd("GET").arg(&key).query_async(&mut *conn).await;
        engine_id.ok()
    }

    pub async fn get_engine_config(&self, name: &str, version: &str) -> Option<EngineConfig> {
        let key = format!("metis.engines.config.{}.{}", name, version);
        let mut conn = self.conn.lock().await;
        let config_json: Result<String, _> =
            redis::cmd("GET").arg(&key).query_async(&mut *conn).await;
        config_json.ok().and_then(|json| serde_json::from_str(&json).ok())
    }
}
