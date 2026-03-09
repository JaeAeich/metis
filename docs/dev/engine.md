# Engine Internals

## Crate Structure

| Crate | Purpose |
| ----- | ------- |
| `api` | Axum HTTP server, route handlers, request validation |
| `engine` | `Engine` trait, shared runtime (NATS subscription, process execution, log capture) |
| `engines/generic` | Generic binary — reads `engine.yaml`, delegates trait methods |
| `common` | Shared domain types (`RunState`, `RunId`, etc.) |
| `telemetry` | Tracing/subscriber init, re-exports `tracing` and `tracing_subscriber` |

## Engine Trait

New engines implement the following methods:

```rust
pub trait Engine: Send + Sync {
    fn new() -> Self;
    async fn get_workflow_results() -> Result<HashMap<Category, Files>>;
    async fn get_task_logs() -> Result<Vec<TaskLog>>;
}
```

Everything else — NATS subscription, process execution, log capture, state transitions — is handled by the shared `engine` crate runtime.

## Adding a Custom Engine

1. Create a new crate that depends on `engine`.
2. Implement the `Engine` trait for result parsing and task log extraction.
3. Call `engine::server::bootstrap(MyEngine::new(), "my-engine-name").await` from `main`.
4. Write an `engine.yaml` — see [Engine Configuration](/engine-configuration) for the full reference.
5. Start the binary with `ENGINE_CONFIG_PATH` pointing at the config file.

For engines that don't need custom parsing, `metis-engine-generic` with an `engine.yaml` is sufficient — no custom binary required.
