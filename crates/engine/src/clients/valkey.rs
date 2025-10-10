use anyhow::Result;
use common::configs::{EngineConfig, ValkeyConfig};
use redis::{AsyncCommands, aio::MultiplexedConnection};
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::{sync::Mutex, time::sleep};

/// Shared Valkey client wrapper
#[derive(Clone)]
pub struct Valkey {
    conn: Arc<Mutex<MultiplexedConnection>>,
    pub engine_config: Arc<EngineConfig>,
    ttl: u64,
}

impl Valkey {
    pub async fn new(
        valkey_config: ValkeyConfig,
        engine_config: EngineConfig,
        ttl: u64,
    ) -> Result<Self> {
        // Build Redis URL
        let mut url = valkey_config.url.clone();
        if let Some(pass) = &valkey_config.password {
            url = url.replace("redis://", &format!("redis://:{}@", pass));
        }

        let client = redis::Client::open(url)?;
        let mut conn = client.get_multiplexed_async_connection().await?;

        // Select DB if specified
        if valkey_config.db > 0 {
            let _: () = redis::cmd("SELECT")
                .arg(valkey_config.db)
                .query_async(&mut conn)
                .await?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            engine_config: Arc::new(engine_config),
            ttl,
        })
    }

    /// Check if this engine is new
    pub async fn am_i_new(&self) -> Result<bool> {
        let key = format!(
            "metis.engines.config.{}.{}",
            self.engine_config.name, self.engine_config.version
        );
        let mut conn = self.conn.lock().await;
        let exists: bool = conn.exists(&key).await?;
        Ok(!exists)
    }

    /// Register engine config in Redis
    pub async fn register(&self) -> Result<()> {
        let config_key = format!(
            "metis.engines.config.{}.{}",
            self.engine_config.name, self.engine_config.version
        );
        let config_json = serde_json::to_string(&*self.engine_config)?;

        let mut pipeline = redis::pipe();
        pipeline
            .cmd("SETEX")
            .arg(&config_key)
            .arg(self.ttl)
            .arg(config_json);

        let mut conn = self.conn.lock().await;
        let _: () = pipeline.query_async(&mut *conn).await?;
        Ok(())
    }

    /// Update heartbeat (called periodically from main tokio task)
    pub async fn heartbeat(&self) -> Result<()> {
        // Efficiently gather CPU and RAM usage
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

        let base = format!(
            "metis.engines.{}.{}.{}",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        let cpu_key = format!("{base}.cpu");
        let ram_key = format!("{base}.ram");

        // Use low-level Cmd pipeline for atomic update
        let mut conn = self.conn.lock().await;
        let mut pipe = redis::pipe();
        pipe.cmd("SETEX")
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

    /// Add a new run entry for this engine
    pub async fn add_run(&self, run_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;

        let engine_runs = format!(
            "metis.engines.{}.{}.{}.runs",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        let metis_runs = format!("metis.runs.{}.{}", run_id, self.engine_config.id);

        let mut pipe = redis::pipe();
        pipe.cmd("INCR").arg(&engine_runs);
        pipe.cmd("SET")
            .arg(&metis_runs)
            .arg(&self.engine_config.id.to_string());

        let _: () = pipe.query_async(&mut *conn).await?;
        Ok(())
    }

    /// Remove a run entry
    pub async fn remove_run(&self, run_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;

        let engine_runs = format!(
            "metis.engines.{}.{}.{}.runs",
            self.engine_config.name, self.engine_config.version, self.engine_config.id
        );
        let metis_runs = format!("metis.runs.{}.{}", run_id, self.engine_config.id);

        let mut pipe = redis::pipe();
        pipe.cmd("DECR").arg(&engine_runs);
        pipe.cmd("DEL").arg(&metis_runs);

        let _: () = pipe.query_async(&mut *conn).await?;
        Ok(())
    }

    pub async fn store_run_pid(&self, run_id: &str, pid: u32) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let _: () = conn.set(key, pid).await?;
        Ok(())
    }

    pub async fn get_run_pid(&self, run_id: &str) -> Result<Option<u32>> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let pid: Option<u32> = conn.get(key).await?;
        Ok(pid)
    }

    pub async fn remove_run_pid(&self, run_id: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let key = format!("metis.runs.{}.pid", run_id);
        let _: () = conn.del(key).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_valkey_register_and_heartbeat() -> Result<()> {
    // Given a running Redis on localhost:6379
    let valkey_config = ValkeyConfig {
        url: "redis://127.0.0.1:6379".into(),
        password: None,
        db: 0,
    };
    use common::configs::{
        Backend, BooleanStyle, DeniedParam, EngineParam, EngineParamsConfig, MatchType, ParamType,
        UnknownBehavior, Validation, WorkflowParamsConfig, WorkflowParamsMethod,
        WorkflowParamsStyle,
    };
    let engine_config = EngineConfig {
        name: "test_engine".into(),
        id: uuid::Uuid::now_v7(),
        version: "v1.0".into(),
        workflow_types: vec!["cwl".into()],
        workflow_type_versions: vec!["1.0".into()],
        command_template: "bash run_workflow.sh --input {input} --output {output}".into(),
        backend: Backend::Local,
        workflow_params: WorkflowParamsConfig {
            style: WorkflowParamsStyle {
                method: WorkflowParamsMethod::Inline,
                prefix: None,
                separator: None,
                key_value_format: None,
                format: None,
                file_path: None,
            },
        },
        engine_params: EngineParamsConfig {
            unknown_params_behavior: UnknownBehavior::Reject,
            validated_params: vec![
                EngineParam {
                    names: vec!["threads".into()],
                    cli_flag: "--threads".into(),
                    param_type: ParamType::Int,
                    default: Some(serde_json::json!(4)),
                    strict_default: false,
                    required: false,
                    sensitive: false,
                    list_separator: None,
                    boolean_style: None,
                    env_var: Some("THREADS".into()),
                    validate: Some(vec![Validation::Custom {
                        rule: "value >= 1 && value <= 64".into(),
                        message: Some("Threads must be between 1 and 64".into()),
                    }]),
                    description: Some("Number of threads to use".into()),
                },
                EngineParam {
                    names: vec!["debug".into()],
                    cli_flag: "--debug".into(),
                    param_type: ParamType::Bool,
                    default: Some(serde_json::json!(false)),
                    strict_default: false,
                    required: false,
                    sensitive: false,
                    list_separator: None,
                    boolean_style: Some(BooleanStyle::Flag),
                    env_var: None,
                    validate: None,
                    description: Some("Enable debug mode".into()),
                },
            ],
        },
        denied_params: vec![DeniedParam {
            pattern: "root_access".into(),
            match_type: MatchType::Exact,
            reason: "Security risk".into(),
        }],
        ignored_params: Some(vec!["legacy_flag".into()]),
    };

    let valkey = Valkey::new(valkey_config, engine_config.clone(), 30).await?;
    let mut conn = valkey.conn.lock().await;

    // Clean previous keys if any
    let config_key = format!(
        "metis.engines.config.{}.{}",
        engine_config.name, engine_config.version
    );
    let cpu_key = format!(
        "metis.engines.{}.{}.{}.cpu",
        engine_config.name, engine_config.version, engine_config.id
    );
    let ram_key = format!(
        "metis.engines.{}.{}.{}.ram",
        engine_config.name, engine_config.version, engine_config.id
    );
    let _: () = conn.del(&config_key).await?;
    let _: () = conn.del(&cpu_key).await?;
    let _: () = conn.del(&ram_key).await?;
    drop(conn);

    // Register engine
    valkey.register().await?;

    // Verify config key is written
    let mut conn = valkey.conn.lock().await;
    let config_key = format!(
        "metis.engines.config.{}.{}",
        engine_config.name, engine_config.version
    );
    let cfg: String = conn.get(&config_key).await?;
    drop(conn);

    assert!(cfg.contains(&engine_config.name));

    // Call heartbeat
    valkey.heartbeat().await?;

    let mut conn = valkey.conn.lock().await;
    let cpu_key = format!(
        "metis.engines.{}.{}.{}.cpu",
        engine_config.name, engine_config.version, engine_config.id
    );
    let ram_key = format!(
        "metis.engines.{}.{}.{}.ram",
        engine_config.name, engine_config.version, engine_config.id
    );
    let cpu: f32 = conn.get(&cpu_key).await?;
    let ram: u64 = conn.get(&ram_key).await?;
    drop(conn);

    assert!(cpu >= 0.0);
    assert!(ram > 0);

    Ok(())
}

#[tokio::test]
async fn test_valkey_add_and_remove_run() -> Result<()> {
    use common::configs::{
        Backend, EngineParamsConfig, UnknownBehavior, WorkflowParamsConfig, WorkflowParamsMethod,
        WorkflowParamsStyle,
    };

    let valkey_config = ValkeyConfig {
        url: "redis://127.0.0.1:6379".into(),
        password: None,
        db: 0,
    };

    let engine_config = EngineConfig {
        name: "test_engine".into(),
        id: uuid::Uuid::now_v7(),
        version: "v1.0".into(),
        workflow_types: vec!["cwl".into()],
        workflow_type_versions: vec!["1.0".into()],
        command_template: "bash run_workflow.sh".into(),
        backend: Backend::Local,
        workflow_params: WorkflowParamsConfig {
            style: WorkflowParamsStyle {
                method: WorkflowParamsMethod::Inline,
                prefix: None,
                separator: None,
                key_value_format: None,
                format: None,
                file_path: None,
            },
        },
        engine_params: EngineParamsConfig {
            unknown_params_behavior: UnknownBehavior::Reject,
            validated_params: vec![],
        },
        denied_params: vec![],
        ignored_params: None,
    };

    let valkey = Valkey::new(valkey_config, engine_config.clone(), 30).await?;
    let run_id = format!("run-{}", chrono::Utc::now().timestamp());

    // Add run
    valkey.add_run(&run_id).await?;

    // Check run entry exists
    let mut conn = valkey.conn.lock().await;
    let run_key = format!("metis.runs.{}.{}", run_id, engine_config.id);
    let run_data: String = conn.get(&run_key).await?;
    drop(conn);

    assert_eq!(run_data, engine_config.id.to_string());

    // Remove run
    valkey.remove_run(&run_id).await?;

    let mut conn = valkey.conn.lock().await;
    let exists: Option<String> = conn.get(&run_key).await?;
    drop(conn);

    assert!(exists.is_none());

    Ok(())
}
