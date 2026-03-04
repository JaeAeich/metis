use std::net::SocketAddr;
use std::time::Duration;

use common::models::State;
use metis_api::docs::docs;
use metis_api::routes::get_router;
use metis_api::{AppState, Config};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let state = AppState::new(&config).await?;

    // Spawn orphan detection background task
    {
        let services = state.services.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let runs = match services.runs.find_active_runs().await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to query active runs for orphan detection");
                        continue;
                    },
                };

                let redis = match services.runs.redis() {
                    Some(r) => r.clone(),
                    None => continue,
                };

                for run in runs {
                    let engine_id = redis.get_assigned_engine(run.id.as_str()).await;

                    if engine_id.is_none() {
                        tracing::warn!(
                            run_id = %run.id,
                            "Orphaned run detected (no assigned engine), marking as SYSTEM_ERROR"
                        );
                        if let Err(e) =
                            services.runs.finalize_orphaned_run(&run.id, State::SystemError).await
                        {
                            tracing::warn!(error = %e, run_id = %run.id, "Failed to finalize orphaned run");
                        }
                    }
                }
            }
        });
    }

    let app = get_router(state).merge(docs());

    let addr: SocketAddr = ([0, 0, 0, 0], config.app_port).into();
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
