use std::sync::Arc;

use common::configs::FullEngineConfig;

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
    let nats = Arc::new(
        Nats::new(
            server_config.nats_url.as_str(),
            &server_config.nats_notification_subject,
            config.engine.clone(),
        )
        .await?,
    );
    let valkey =
        Arc::new(Valkey::new(server_config.redis_url.as_str(), config.engine.clone()).await?);
    let db = Arc::new(Db::new(server_config.database_url.as_str()).await?);

    let runtime = Arc::new(EngineRuntime::new(
        Arc::new(engine),
        config,
        Some(nats),
        Some(valkey),
        Some(db),
        false,
    ));
    runtime.start().await
}
