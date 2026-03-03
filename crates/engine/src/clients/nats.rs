use crate::error::{EngineError, EngineResult};
use async_nats::{Client, Subscriber};
use common::configs::{EngineConfig, NatsConfig};
use std::sync::Arc;

pub struct Nats {
    pub client: Arc<Client>,
    pub engine_config: EngineConfig,
    pub notification_subject: String,
}

impl Nats {
    pub async fn new(nats_config: &NatsConfig, engine_config: EngineConfig) -> EngineResult<Self> {
        let client = async_nats::connect(&nats_config.url)
            .await
            .map_err(|e| EngineError::Network(e.to_string()))?;
        Ok(Self {
            client: Arc::new(client),
            engine_config,
            notification_subject: nats_config.notification_subject.clone(),
        })
    }

    pub fn run_topics(&self) -> Vec<String> {
        self.engine_config
            .workflow_types
            .iter()
            .flat_map(|language| {
                self.engine_config
                    .workflow_type_versions
                    .iter()
                    .map(move |version| {
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
                .subscribe(topic)
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
