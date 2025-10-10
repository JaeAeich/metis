// use async_nats;
// use chrono::Utc;
// use common::configs::{EngineConfig, NatsConfig};
// use common::utils::connect_valkey;
// use redis::AsyncCommands;
// use serde_json::json;
// pub struct Register {
//     engine_config: EngineConfig,
//     client: async_nats::Client,
//     valkey: redis::Client,
// }

// impl Register {
//     pub async fn new(
//         engine_config: EngineConfig,
//         redis_client: redis::Client,
//         nats_client: async_nats::Client,
//     ) -> Result<Self, async_nats::Error> {
//         Ok(Self {
//             engine_config,
//             client: nats_client,
//             valkey: redis_client,
//         })
//     }

//     /// If the engine is already registered, then return the NATS topic to connect to.
//     /// Else, register the engine in Valkey, send a notification via NATS,
//     /// and return the NATS topic to connect to.
//     pub async fn register(&self) -> Result<String, Box<dyn std::error::Error>> {
//         let mut conn = connect_valkey(&self.valkey).await?;

//         let engine_key = format!("engine:{}:config", self.engine_config.id);
//         let exists: bool = conn.exists(&engine_key).await?;

//         let nats_topic = format!("engine.{}", self.engine_config.id);

//         if !exists {
//             let cfg_json = serde_json::to_string(&self.engine_config)?;
//             let _: () = conn.set(&engine_key, cfg_json).await?;
//             let _: () = conn
//                 .sadd("engines:registered", &self.engine_config.id)
//                 .await?;

//             // Notify the system about the new engine registration
//             let msg = json!({
//                 "event": "engine_registered",
//                 "engine_id": self.engine_config.id,
//                 "timestamp": Utc::now().to_rfc3339(),
//             });
//             self.client
//                 .publish("engine.events", msg.to_string().into())
//                 .await?;
//             self.client.flush().await?;
//         } else {
//             // Refresh the registration timestamp if it already exists
//             let key_last_seen = format!("engine:{}:last_seen", self.engine_config.id);
//             let _: () = conn
//                 .set_ex(&key_last_seen, Utc::now().timestamp(), 60)
//                 .await?;
//         }

//         Ok(nats_topic)
//     }

//     /// Sends a heartbeat (publishes to NATS and updates Valkey)
//     pub async fn heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
//         let mut conn = self.valkey.get_async_connection().await?;

//         // Example system stats
//         let cpu = self.get_cpu_usage().await;
//         let status = json!({
//             "engine_id": self.engine_config.id,
//             "cpu": cpu,
//             "timestamp": Utc::now().to_rfc3339(),
//         });

//         let key_status = format!("engine:{}:status", self.engine_config.id);
//         let _: () = conn.set_ex(key_status, status.to_string(), 30).await?;

//         // Publish heartbeat to NATS
//         let topic = format!("engine.{}.heartbeat", self.engine_config.id);
//         self.client
//             .publish(topic, status.to_string().into())
//             .await?;

//         Ok(())
//     }

//     /// Subscribes to a NATS subject (optional)
//     pub async fn subscribe(
//         &self,
//         subject: &str,
//     ) -> Result<async_nats::Subscriber, async_nats::Error> {
//         self.client.subscribe(subject.to_string()).await
//     }

//     /// Simple CPU usage fetcher (placeholder)
//     async fn get_cpu_usage(&self) -> f32 {
//         use sysinfo::{CpuExt, System, SystemExt};
//         let mut sys = System::new_all();
//         sys.refresh_cpu();
//         sys.global_cpu_info().cpu_usage()
//     }
// }
