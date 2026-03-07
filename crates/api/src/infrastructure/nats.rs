use common::models::RunRequestMessage;

pub struct NatsPublisher {
    client: async_nats::Client,
}

impl NatsPublisher {
    pub async fn new(url: &str) -> Result<Self, async_nats::ConnectError> {
        let client = if let Ok(parsed) = url::Url::parse(url) {
            if !parsed.username().is_empty() {
                async_nats::ConnectOptions::with_user_and_password(
                    parsed.username().to_string(),
                    parsed.password().unwrap_or("").to_string(),
                )
                .connect(url)
                .await?
            } else {
                async_nats::connect(url).await?
            }
        } else {
            async_nats::connect(url).await?
        };
        Ok(Self { client })
    }

    pub async fn publish_run(&self, topic: &str, msg: &RunRequestMessage) -> Result<(), String> {
        let payload = serde_json::to_vec(msg).map_err(|e| format!("Serialization error: {}", e))?;
        self.client
            .publish(topic.to_string(), payload.into())
            .await
            .map_err(|e| format!("NATS publish error: {}", e))?;
        Ok(())
    }

    pub async fn publish_cancel(&self, engine_id: &str, run_id: &str) -> Result<(), String> {
        let topic = common::keys::nats_cancel_subject(engine_id);
        let payload = serde_json::json!({ "run_id": run_id }).to_string().into_bytes();
        self.client
            .publish(topic, payload.into())
            .await
            .map_err(|e| format!("NATS publish error: {}", e))?;
        Ok(())
    }
}
