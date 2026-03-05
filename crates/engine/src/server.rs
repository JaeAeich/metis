use std::sync::Arc;

use tokio::try_join;

use crate::clients::{Db, Nats, Valkey};
use crate::config::ServerConfig;
use crate::engine::Engine;
use crate::error::EngineResult;
use crate::runtime::EngineRuntime;

pub async fn bootstrap<E>(engine: E) -> EngineResult<()>
where
    E: Engine + 'static,
{
    let config = crate::config::load_engine_config().await?;
    let server_config = ServerConfig::from_env().map_err(|e| {
        crate::error::EngineError::Config(format!("Failed to load server config: {}", e))
    })?;

    let (nats, valkey, db) = try_join!(
        async {
            Ok::<_, crate::error::EngineError>(Arc::new(
                Nats::new(
                    server_config.nats_url.as_str(),
                    &server_config.nats_notification_subject,
                    config.engine.clone(),
                )
                .await?,
            ))
        },
        async {
            Ok::<_, crate::error::EngineError>(Arc::new(
                Valkey::new(server_config.redis_url.as_str(), config.engine.clone()).await?,
            ))
        },
        async {
            Ok::<_, crate::error::EngineError>(Arc::new(
                Db::new(server_config.database_url.as_str()).await?,
            ))
        },
    )?;

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
