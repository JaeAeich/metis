use std::sync::Arc;

use common::configs::{DatabaseConfig, FullEngineConfig, NatsConfig, ValkeyConfig};
use tracing::warn;

use crate::clients::{Db, Nats, Valkey};
use crate::engine::Engine;
use crate::error::EngineResult;
use crate::runtime::EngineRuntime;

pub async fn bootstrap<E>(
    engine: E,
    config: FullEngineConfig,
    nats_config: NatsConfig,
    valkey_config: ValkeyConfig,
    db_config: Option<DatabaseConfig>,
) -> EngineResult<()>
where
    E: Engine + 'static,
{
    let nats = Arc::new(Nats::new(&nats_config, config.engine.clone()).await?);
    let valkey = Arc::new(Valkey::new(valkey_config, config.engine.clone(), 60).await?);
    let db = match db_config {
        Some(cfg) => match Db::new(&cfg).await {
            Ok(db) => Some(Arc::new(db)),
            Err(e) => {
                warn!(error = %e, "Database unavailable — run persistence disabled");
                None
            },
        },
        None => {
            warn!("DATABASE_URL not set — run persistence disabled");
            None
        },
    };

    let runtime = Arc::new(EngineRuntime::new(
        Arc::new(engine),
        config,
        Some(nats),
        Some(valkey),
        db,
        false,
    ));
    runtime.start().await
}
