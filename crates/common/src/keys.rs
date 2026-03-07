//! Centralized key/subject definitions for Valkey and NATS.
//!
//! All format strings are defined here. Use the provided functions
//! instead of constructing keys/subjects inline.

// ─── Valkey Keys
// ──────────────────────────────────────────────────────────────

/// Stores the serialized `EngineConfig` for a registered engine.
/// Used by both engine (registration) and api (engine discovery).
pub fn valkey_engine_config(name: &str, version: &str) -> String {
    format!("metis.engines.config.{}.{}", name, version)
}

/// SCAN glob pattern to list all registered engine configs.
pub fn valkey_engine_config_pattern() -> &'static str {
    "metis.engines.config.*.*"
}

/// Reverse index: maps a run ID to the engine ID that owns it.
/// Written when a run starts, deleted when it ends.
pub fn valkey_run_engine(run_id: &str) -> String {
    format!("metis.runs.{}.engine", run_id)
}

/// Stores the OS process ID (PID) for a running workflow process.
pub fn valkey_run_pid(run_id: &str) -> String {
    format!("metis.runs.{}.pid", run_id)
}

// ─── NATS Subjects
// ────────────────────────────────────────────────────────────

/// Subject for dispatching a new run request to a compatible engine.
/// Engines subscribe to subjects matching their capabilities.
pub fn nats_run_subject(
    engine: &str,
    engine_version: &str,
    workflow_type: &str,
    workflow_type_version: &str,
) -> String {
    format!(
        "metis.runs.{}.{}.{}.{}",
        engine, engine_version, workflow_type, workflow_type_version
    )
}

/// Subject for sending a cancel command to a specific engine instance.
pub fn nats_cancel_subject(engine_id: impl std::fmt::Display) -> String {
    format!("metis.cancel.{}", engine_id)
}

/// Default subject on which engines publish run status notifications.
pub fn nats_notification_subject() -> &'static str {
    "metis.notification"
}
