use std::sync::Arc;

use async_nats::{Client, Subscriber};
use common::configs::EngineConfig;

use crate::error::{EngineError, EngineResult};

pub struct Nats {
    pub client: Arc<Client>,
    pub engine_config: EngineConfig,
    pub notification_subject: String,
}

impl Nats {
    pub async fn new(
        url: &str,
        notification_subject: &str,
        engine_config: EngineConfig,
    ) -> EngineResult<Self> {
        let client = if let Ok(parsed) = url::Url::parse(url) {
            if !parsed.username().is_empty() {
                async_nats::ConnectOptions::with_user_and_password(
                    parsed.username().to_string(),
                    parsed.password().unwrap_or("").to_string(),
                )
                .connect(url)
                .await
            } else {
                async_nats::connect(url).await
            }
        } else {
            async_nats::connect(url).await
        }
        .map_err(|e| EngineError::Network(e.to_string()))?;

        Ok(Self {
            client: Arc::new(client),
            engine_config,
            notification_subject: notification_subject.to_string(),
        })
    }

    pub fn run_topics(&self) -> Vec<String> {
        self.engine_config
            .workflow_types
            .iter()
            .flat_map(|language| {
                self.engine_config.workflow_type_versions.iter().map(move |version| {
                    format!(
                        "metis.runs.{}.{}.{}.{}",
                        self.engine_config.name, self.engine_config.version, language, version
                    )
                })
            })
            .collect()
    }

    pub fn cancel_topic(&self) -> String {
        format!("metis.cancel.{}", self.engine_config.id)
    }

    pub async fn subscribe_runs(&self) -> EngineResult<Vec<Subscriber>> {
        let mut subs = Vec::new();
        for topic in self.run_topics() {
            let sub = self
                .client
                .queue_subscribe(topic, "engine-workers".to_string())
                .await
                .map_err(|e| EngineError::Network(e.to_string()))?;
            subs.push(sub);
        }
        Ok(subs)
    }

    pub async fn subscribe_cancel(&self) -> EngineResult<Subscriber> {
        let topic = self.cancel_topic();
        let sub = self
            .client
            .subscribe(topic)
            .await
            .map_err(|e| EngineError::Network(e.to_string()))?;
        Ok(sub)
    }

    pub async fn publish_notification(&self, payload: impl Into<Vec<u8>>) -> EngineResult<()> {
        self.client
            .publish(self.notification_subject.clone(), payload.into().into())
            .await
            .map_err(|e| EngineError::Network(e.to_string()))?;
        Ok(())
    }
}
