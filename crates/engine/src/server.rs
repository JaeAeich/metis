use crate::engine::{Engine, NoopEngine};
use crate::runtime::EngineRuntime;
use common::configs::{FullEngineConfig, NatsConfig, ValkeyConfig};
use std::sync::Arc;

pub async fn bootstrap<E>(
    engine: E,
    config: FullEngineConfig,
    nats_config: NatsConfig,
    valkey_config: ValkeyConfig,
) -> anyhow::Result<()>
where
    E: Engine + 'static,
{
    let runtime = Arc::new(
        EngineRuntime::new(
            Arc::new(engine),
            config,
            Some(nats_config),
            Some(valkey_config),
            false,  // dry_run = false for server mode
        )
        .await?,
    );
    runtime.start().await?;
    Ok(())
}

pub async fn start_server(
    config: FullEngineConfig,
    nats_config: NatsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let valkey_config = ValkeyConfig::from_env();
    bootstrap(NoopEngine, config, nats_config, valkey_config).await?;
    Ok(())
}
