use std::sync::Arc;

use common::configs::EngineConfig;
use common::keys;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use tokio::sync::Mutex;

use crate::error::EngineResult;

#[derive(Clone)]
pub struct Valkey {
    conn: Arc<Mutex<MultiplexedConnection>>,
    pub engine_config: Arc<EngineConfig>,
}

impl Valkey {
    pub async fn new(url: &str, engine_config: EngineConfig) -> EngineResult<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            engine_config: Arc::new(engine_config),
        })
    }

    pub async fn am_i_new(&self) -> EngineResult<bool> {
        let key = keys::valkey_engine_config(&self.engine_config.name, &self.engine_config.version);
        let mut conn = self.conn.lock().await;
        let exists: bool = conn.exists(&key).await?;
        Ok(!exists)
    }

    pub async fn register(&self) -> EngineResult<()> {
        let config_key =
            keys::valkey_engine_config(&self.engine_config.name, &self.engine_config.version);
        let config_json = serde_json::to_string(&*self.engine_config)?;

        let mut conn = self.conn.lock().await;
        let _: () = redis::cmd("SET")
            .arg(&config_key)
            .arg(config_json)
            .query_async(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn add_run(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        // Direct reverse-index for O(1) engine lookup during cancel
        let run_engine_index = keys::valkey_run_engine(run_id);
        let _: () = redis::cmd("SET")
            .arg(&run_engine_index)
            .arg(self.engine_config.id.to_string())
            .query_async(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn remove_run(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        let run_engine_index = keys::valkey_run_engine(run_id);
        let _: () = redis::cmd("DEL").arg(&run_engine_index).query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn store_run_pid(&self, run_id: &str, pid: u32) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        let key = keys::valkey_run_pid(run_id);
        let _: () = conn.set(key, pid).await?;
        Ok(())
    }

    pub async fn get_run_pid(&self, run_id: &str) -> EngineResult<Option<u32>> {
        let mut conn = self.conn.lock().await;
        let key = keys::valkey_run_pid(run_id);
        let pid: Option<u32> = conn.get(key).await?;
        Ok(pid)
    }

    pub async fn remove_run_pid(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        let key = keys::valkey_run_pid(run_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_valkey_register() -> EngineResult<()> {
    let valkey = Valkey::new(
        "redis://127.0.0.1:6379",
        common::configs::EngineConfig {
            name: "test_engine".into(),
            id: uuid::Uuid::now_v7(),
            version: "v1.0".into(),
            workflow_types: vec!["cwl".into()],
            workflow_type_versions: vec!["1.0".into()],
            command_template: "bash run_workflow.sh".into(),
            backend: common::configs::Backend::Local,
            workflow_params: common::configs::WorkflowParamsConfig {
                style: common::configs::WorkflowParamsStyle {
                    method: common::configs::WorkflowParamsMethod::Inline,
                    prefix: None,
                    separator: None,
                    key_value_format: None,
                    format: None,
                    file_path: None,
                    params_file_flag: None,
                },
            },
            engine_params: common::configs::EngineParamsConfig {
                unknown_params_behavior: common::configs::UnknownBehavior::Reject,
                validated_params: vec![],
            },
            denied_params: vec![],
            ignored_params: None,
        },
    )
    .await?;

    valkey.register().await?;

    Ok(())
}

#[tokio::test]
async fn test_valkey_add_and_remove_run() -> EngineResult<()> {
    let engine_config = common::configs::EngineConfig {
        name: "test_engine".into(),
        id: uuid::Uuid::now_v7(),
        version: "v1.0".into(),
        workflow_types: vec!["cwl".into()],
        workflow_type_versions: vec!["1.0".into()],
        command_template: "bash run_workflow.sh".into(),
        backend: common::configs::Backend::Local,
        workflow_params: common::configs::WorkflowParamsConfig {
            style: common::configs::WorkflowParamsStyle {
                method: common::configs::WorkflowParamsMethod::Inline,
                prefix: None,
                separator: None,
                key_value_format: None,
                format: None,
                file_path: None,
                params_file_flag: None,
            },
        },
        engine_params: common::configs::EngineParamsConfig {
            unknown_params_behavior: common::configs::UnknownBehavior::Reject,
            validated_params: vec![],
        },
        denied_params: vec![],
        ignored_params: None,
    };

    let valkey = Valkey::new("redis://127.0.0.1:6379", engine_config.clone()).await?;
    let run_id = format!("run-{}", chrono::Utc::now().timestamp());

    valkey.add_run(&run_id).await?;

    let mut conn = valkey.conn.lock().await;
    let run_key = keys::valkey_run_engine(&run_id);
    let run_data: String = conn.get(&run_key).await?;
    drop(conn);

    assert_eq!(run_data, engine_config.id.to_string());

    valkey.remove_run(&run_id).await?;

    let mut conn = valkey.conn.lock().await;
    let exists: Option<String> = conn.get(&run_key).await?;
    drop(conn);

    assert!(exists.is_none());

    Ok(())
}
