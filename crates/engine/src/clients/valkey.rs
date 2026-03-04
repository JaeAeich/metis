use std::sync::Arc;

use common::configs::EngineConfig;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::error::EngineResult;

#[derive(Clone)]
pub struct Valkey {
    conn: Arc<Mutex<MultiplexedConnection>>,
    pub engine_config: Arc<EngineConfig>,
    ttl: u64,
}

impl Valkey {
    pub async fn new(url: &str, engine_config: EngineConfig, ttl: u64) -> EngineResult<Self> {
        let client = redis::Client::open(url)?;
        let conn = client.get_multiplexed_async_connection().await?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            engine_config: Arc::new(engine_config),
            ttl,
        })
    }

    pub async fn am_i_new(&self) -> EngineResult<bool> {
        let key = format!(
            "metis.engines.config.{}.{}",
            self.engine_config.name, self.engine_config.version
        );
        let mut conn = self.conn.lock().await;
        let exists: bool = conn.exists(&key).await?;
        Ok(!exists)
    }

    pub async fn register(&self) -> EngineResult<()> {
        let config_key = format!(
            "metis.engines.config.{}.{}",
            self.engine_config.name, self.engine_config.version
        );
        let config_json = serde_json::to_string(&*self.engine_config)?;

        let mut pipeline = redis::pipe();
        pipeline.cmd("SETEX").arg(&config_key).arg(self.ttl).arg(config_json);

        let mut conn = self.conn.lock().await;
        let _: () = pipeline.query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn heartbeat(&self) -> EngineResult<()> {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpu_usage = sys.global_cpu_usage();
        let used_ram = sys.total_memory() - sys.available_memory();

        let config_key = format!(
            "metis.engines.config.{}.{}",
            self.engine_config.name, self.engine_config.version
        );
        let base = format!(
            "metis.engines.{}.{}.{}",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        let cpu_key = format!("{base}.cpu");
        let ram_key = format!("{base}.ram");

        let mut conn = self.conn.lock().await;
        let mut pipe = redis::pipe();
        pipe.cmd("EXPIRE")
            .arg(&config_key)
            .arg(self.ttl)
            .cmd("SETEX")
            .arg(&cpu_key)
            .arg(self.ttl)
            .arg(cpu_usage)
            .cmd("SETEX")
            .arg(&ram_key)
            .arg(self.ttl)
            .arg(used_ram);

        let _: () = pipe.query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn add_run(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;

        let engine_runs = format!(
            "metis.engines.{}.{}.{}.runs",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        // Direct reverse-index for O(1) engine lookup during cancel
        let run_engine_index = format!("metis.runs.{}.engine", run_id);

        let mut pipe = redis::pipe();
        pipe.cmd("INCR").arg(&engine_runs);
        pipe.cmd("SET").arg(&run_engine_index).arg(self.engine_config.id.to_string());

        let _: () = pipe.query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn remove_run(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;

        let engine_runs = format!(
            "metis.engines.{}.{}.{}.runs",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        let run_engine_index = format!("metis.runs.{}.engine", run_id);

        let mut pipe = redis::pipe();
        pipe.cmd("DECR").arg(&engine_runs);
        pipe.cmd("DEL").arg(&run_engine_index);

        let _: () = pipe.query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn store_run_pid(&self, run_id: &str, pid: u32) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let _: () = conn.set(key, pid).await?;
        Ok(())
    }

    pub async fn get_run_pid(&self, run_id: &str) -> EngineResult<Option<u32>> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let pid: Option<u32> = conn.get(key).await?;
        Ok(pid)
    }

    pub async fn remove_run_pid(&self, run_id: &str) -> EngineResult<()> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_valkey_register_and_heartbeat() -> EngineResult<()> {
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
        30,
    )
    .await?;

    valkey.register().await?;
    valkey.heartbeat().await?;

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

    let valkey = Valkey::new("redis://127.0.0.1:6379", engine_config.clone(), 30).await?;
    let run_id = format!("run-{}", chrono::Utc::now().timestamp());

    valkey.add_run(&run_id).await?;

    let mut conn = valkey.conn.lock().await;
    let run_key = format!("metis.runs.{}.engine", run_id);
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
