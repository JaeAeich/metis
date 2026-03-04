use std::sync::Arc;

use common::configs::FullEngineConfig;
use tracing::warn;

use crate::clients::{Db, Nats, Valkey};
use crate::config::ServerConfig;
use crate::engine::Engine;
use crate::error::EngineResult;
use crate::runtime::EngineRuntime;

pub async fn bootstrap<E>(
    engine: E,
    config: FullEngineConfig,
    server_config: ServerConfig,
) -> EngineResult<()>
where
    E: Engine + 'static,
{
    let nats = match &server_config.nats_url {
        Some(url) if !url.is_empty() => {
            match Nats::new(url, &server_config.nats_notification_subject, config.engine.clone())
                .await
            {
                Ok(n) => Some(Arc::new(n)),
                Err(e) => {
                    warn!(error = %e, nats_url = %url, "NATS unavailable — messaging disabled");
                    None
                },
            }
        },
        _ => {
            warn!("NATS_URL not set — messaging disabled");
            None
        },
    };

    let valkey = match &server_config.redis_url {
        Some(url) if !url.is_empty() => match Valkey::new(url, config.engine.clone(), 60).await {
            Ok(v) => Some(Arc::new(v)),
            Err(e) => {
                warn!(error = %e, "Redis unavailable — engine tracking disabled");
                None
            },
        },
        _ => {
            warn!("REDIS_URL not set — engine tracking disabled");
            None
        },
    };

    let db = match &server_config.database_url {
        Some(url) if !url.is_empty() => match Db::new(url).await {
            Ok(db) => Some(Arc::new(db)),
            Err(e) => {
                warn!(error = %e, "Database unavailable — run persistence disabled");
                None
            },
        },
        _ => {
            warn!("DATABASE_URL not set — run persistence disabled");
            None
        },
    };

    let runtime = Arc::new(EngineRuntime::new(Arc::new(engine), config, nats, valkey, db, false));
    runtime.start().await
}
