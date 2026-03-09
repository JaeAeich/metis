# Tracing

Metis uses [`tracing`](https://docs.rs/tracing) for structured logs and spans. Both `metis-api` and engine binaries share the same setup via `telemetry::init_tracing(service_name)`.

## Log Format

Default output is human-readable text. Set `LOG_FORMAT=json` for newline-delimited JSON — one object per line with `timestamp`, `level`, `span`, and `fields`.

```bash
LOG_FORMAT=json ./metis-api
```

## Log Level

Controlled via `RUST_LOG`:

```bash
RUST_LOG=info              # default
RUST_LOG=debug             # verbose
RUST_LOG=metis_api=debug   # per-crate
```

## Key Spans

| Span              | Crate  | Notable fields                                                                        |
| ----------------- | ------ | ------------------------------------------------------------------------------------- |
| `create_run`      | API    | `run_id`, `user_id`, `workflow_type`, `workflow_engine`                               |
| `request_cancel`  | API    | `run_id`                                                                              |
| `workflow_run`    | Engine | `run_id`, `user_id`, `workflow_type`, `state`, `duration_ms`, `api_trace_id`, `error` |
| `workflow_cancel` | Engine | `run_id`                                                                              |

`state` on `workflow_run` is recorded at each phase: `initializing → building → executing → running → parsing → updating_db → cleanup`. `api_trace_id` links the engine span back to the originating API span.

## NATS Trace Correlation

When the API dispatches a run it embeds the current span ID as `trace_id` in the NATS message. The engine records it as `api_trace_id` on the `workflow_run` span, enabling cross-service correlation in any log aggregator without full distributed tracing.

## OTEL Export

Set `OTEL_EXPORTER_OTLP_ENDPOINT` to export spans to any OTLP-compatible collector (SigNoz, Jaeger, Tempo, Datadog, etc.). The layer activates automatically; no other config is needed.

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://signoz-collector:4317 ./metis-api
```

## Dependency

`tracing` and `tracing_subscriber` are not direct deps of `api` or `engine` — they are re-exported from `telemetry`:

```rust
// crates/telemetry/src/lib.rs
pub use tracing;
pub use tracing_subscriber;
```

Import via `telemetry::tracing` rather than adding a direct dep. The exception is `api`, which keeps a direct `tracing` dep because the `#[tracing::instrument]` proc-macro resolves the crate by name at expansion time.
