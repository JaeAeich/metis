use std::collections::HashMap;
use std::sync::Arc;

use common::configs::EngineConfig;
use common::models::EngineInstance;
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

    pub async fn list_engine_configs(&self) -> Vec<EngineConfig> {
        // Phase 1: collect all keys via SCAN
        let keys = self.scan_keys("metis.engines.config.*.*").await;
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

    pub async fn list_running_engines(&self) -> Vec<EngineInstance> {
        // Phase 1: collect cpu keys via SCAN to find running instances
        let cpu_keys = self.scan_keys("metis.engines.*:*:*.cpu").await;
        if cpu_keys.is_empty() {
            return vec![];
        }

        // Parse instance identifiers and deduplicate
        let mut unique_instances: HashMap<String, (String, String, String)> = HashMap::new();
        for key in &cpu_keys {
            if let Some((name, version, id)) = Self::parse_engine_instance_key(key) {
                let instance_key = format!("{}:{}:{}", name, version, id);
                unique_instances.entry(instance_key).or_insert((name, version, id));
            }
        }

        if unique_instances.is_empty() {
            return vec![];
        }

        // Phase 2: batch-fetch all metrics + config keys via pipeline
        let mut metric_keys: Vec<String> = Vec::new();
        let mut config_keys: Vec<String> = Vec::new();
        // Maintain order for result mapping
        let ordered: Vec<(String, String, String, String)> = unique_instances
            .into_values()
            .map(|(name, version, id)| {
                let base = format!("metis.engines.{}:{}:{}", name, version, id);
                let cfg_key = format!("metis.engines.config.{}.{}", name, version);
                metric_keys.push(format!("{base}.cpu"));
                metric_keys.push(format!("{base}.ram"));
                metric_keys.push(format!("{base}.runs"));
                config_keys.push(cfg_key.clone());
                (name, version, id, cfg_key)
            })
            .collect();

        let all_keys: Vec<&str> = metric_keys
            .iter()
            .map(|s| s.as_str())
            .chain(config_keys.iter().map(|s| s.as_str()))
            .collect();

        let mut conn = self.conn.lock().await;
        let values: Vec<Option<String>> = redis::cmd("MGET")
            .arg(&all_keys)
            .query_async(&mut *conn)
            .await
            .unwrap_or_default();
        drop(conn);

        let n = ordered.len();
        let mut configs_cache: HashMap<String, Option<EngineConfig>> = HashMap::new();

        // Config values start after n*3 metric values
        for (i, (_, _, _, cfg_key)) in ordered.iter().enumerate() {
            let cfg_val = values
                .get(n * 3 + i)
                .and_then(|v| v.as_deref())
                .and_then(|json| serde_json::from_str::<EngineConfig>(json).ok());
            configs_cache.insert(cfg_key.clone(), cfg_val);
        }

        ordered
            .into_iter()
            .enumerate()
            .map(|(i, (name, version, id, cfg_key))| {
                let base_idx = i * 3;
                let cpu: Option<f32> =
                    values.get(base_idx).and_then(|v| v.as_deref()).and_then(|s| s.parse().ok());
                let ram: Option<u64> = values
                    .get(base_idx + 1)
                    .and_then(|v| v.as_deref())
                    .and_then(|s| s.parse().ok());
                let runs: Option<i64> = values
                    .get(base_idx + 2)
                    .and_then(|v| v.as_deref())
                    .and_then(|s| s.parse().ok());

                let config = configs_cache.get(&cfg_key).and_then(|c| c.as_ref());
                let (workflow_types, workflow_type_versions, backend) =
                    config.map_or((vec![], vec![], "unknown".to_string()), |c| {
                        (
                            c.workflow_types.clone(),
                            c.workflow_type_versions.clone(),
                            format!("{:?}", c.backend).to_lowercase(),
                        )
                    });

                EngineInstance {
                    name,
                    version,
                    id,
                    workflow_types,
                    workflow_type_versions,
                    backend,
                    cpu_usage_percent: cpu,
                    ram_used_mb: ram.map(|b| b / 1024 / 1024),
                    runs_count: runs,
                }
            })
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

    fn parse_engine_instance_key(key: &str) -> Option<(String, String, String)> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() >= 4 && parts[0] == "metis" && parts[1] == "engines" {
            let instance_part = parts[2];
            let instance_parts: Vec<&str> = instance_part.split(':').collect();
            if instance_parts.len() == 3 {
                let name = instance_parts[0].to_string();
                let version = instance_parts[1].to_string();
                let id = instance_parts[2].to_string();
                return Some((name, version, id));
            }
        }
        None
    }
}
