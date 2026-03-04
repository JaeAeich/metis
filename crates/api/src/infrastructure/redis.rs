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
        let mut conn = self.conn.lock().await;
        let pattern = "metis.engines.config.*.*";
        let mut configs = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let result: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .query_async(&mut *conn)
                .await;

            match result {
                Ok((new_cursor, keys)) => {
                    for key in keys {
                        let config_json: Result<String, _> =
                            redis::cmd("GET").arg(&key).query_async(&mut *conn).await;
                        if let Some(json) = config_json.ok()
                            && let Ok(config) = serde_json::from_str::<EngineConfig>(&json)
                        {
                            configs.push(config);
                        }
                    }
                    cursor = new_cursor;
                    if cursor == 0 {
                        break;
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to scan engine configs");
                    break;
                },
            }
        }

        configs
    }

    pub async fn list_running_engines(&self) -> Vec<EngineInstance> {
        let mut conn = self.conn.lock().await;
        let pattern = "metis.engines.*:*:*.cpu";
        let mut cursor: u64 = 0;
        let mut instances: HashMap<String, EngineInstance> = HashMap::new();
        let mut configs_cache: HashMap<(String, String), Option<EngineConfig>> = HashMap::new();

        loop {
            let result: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .query_async(&mut *conn)
                .await;

            match result {
                Ok((new_cursor, keys)) => {
                    for key in keys {
                        if let Some((name, version, id)) = Self::parse_engine_instance_key(&key) {
                            let instance_key = format!("{}:{}:{}", name, version, id);
                            if let std::collections::hash_map::Entry::Vacant(e) =
                                instances.entry(instance_key)
                            {
                                let base = format!("metis.engines.{}:{}:{}", name, version, id);
                                let cpu_key = format!("{}.cpu", base);
                                let ram_key = format!("{}.ram", base);
                                let runs_key = format!("{}.runs", base);

                                let cpu: Option<f32> = redis::cmd("GET")
                                    .arg(&cpu_key)
                                    .query_async(&mut *conn)
                                    .await
                                    .ok();
                                let ram: Option<u64> = redis::cmd("GET")
                                    .arg(&ram_key)
                                    .query_async(&mut *conn)
                                    .await
                                    .ok();
                                let runs: Option<i64> = redis::cmd("GET")
                                    .arg(&runs_key)
                                    .query_async(&mut *conn)
                                    .await
                                    .ok();

                                let config_key = (name.clone(), version.clone());
                                let config = if let Some(c) = configs_cache.get(&config_key) {
                                    c.clone()
                                } else {
                                    let cfg_key =
                                        format!("metis.engines.config.{}.{}", name, version);
                                    let config_json: Result<String, _> = redis::cmd("GET")
                                        .arg(&cfg_key)
                                        .query_async(&mut *conn)
                                        .await;
                                    let cfg = config_json
                                        .ok()
                                        .and_then(|json| serde_json::from_str(&json).ok());
                                    configs_cache.insert(config_key.clone(), cfg.clone());
                                    cfg
                                };

                                let (workflow_types, workflow_type_versions, backend) = config
                                    .as_ref()
                                    .map_or((vec![], vec![], "unknown".to_string()), |c| {
                                        (
                                            c.workflow_types.clone(),
                                            c.workflow_type_versions.clone(),
                                            format!("{:?}", c.backend).to_lowercase(),
                                        )
                                    });

                                e.insert(EngineInstance {
                                    name,
                                    version,
                                    id,
                                    workflow_types,
                                    workflow_type_versions,
                                    backend,
                                    cpu_usage_percent: cpu,
                                    ram_used_mb: ram.map(|b| b / 1024 / 1024),
                                    runs_count: runs,
                                });
                            }
                        }
                    }
                    cursor = new_cursor;
                    if cursor == 0 {
                        break;
                    }
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to scan running engines");
                    break;
                },
            }
        }

        instances.into_values().collect()
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
