use std::sync::Arc;

use common::configs::{FullEngineConfig, NatsConfig, ValkeyConfig};

use crate::engine::Engine;
use crate::error::EngineResult;
use crate::runtime::EngineRuntime;

pub async fn bootstrap<E>(
    engine: E,
    config: FullEngineConfig,
    nats_config: NatsConfig,
    valkey_config: ValkeyConfig,
) -> EngineResult<()>
where
    E: Engine + 'static,
{
    let runtime = Arc::new(
        EngineRuntime::new(
            Arc::new(engine),
            config,
            Some(nats_config),
            Some(valkey_config),
            false, // dry_run = false for server mode
        )
        .await?,
    );
    runtime.start().await?;
    Ok(())
}
