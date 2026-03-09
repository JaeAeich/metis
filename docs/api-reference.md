# API Reference

Metis closely follows the [GA4GH WES 1.1.0](https://ga4gh.github.io/workflow-execution-service-schemas/) standard,
extending it with SSE streaming, log pagination, and soft-delete.

::: note live documentation
Interactive API documentation is available at `http://<host>:<port>/docs`
when the API is running. It contains the complete and most up-to-date
request and response schemas. This page is provided for reference only
and may lag behind the live documentation.
:::

## Base URL

```text
http://localhost:8080
```

## What You Can Do

| Endpoint | Description |
| -------- | ----------- |
| `GET /service-info` | List registered engines and supported workflow types |
| `GET /runs` | List runs with pagination and state filtering |
| `POST /runs` | Submit a new workflow run |
| `GET /runs/{id}` | Full run details, logs, and outputs |
| `GET /runs/{id}/status` | Current run state (lightweight poll) |
| `GET /runs/{id}/status/stream` | SSE stream of state transitions in real time |
| `POST /runs/{id}/cancel` | Cancel a running workflow |
| `DELETE /runs/{id}` | Soft-delete a run (record preserved in DB) |
| `GET /runs/{id}/logs` | Paginated stdout/stderr lines |
| `GET /runs/{id}/logs/stream` | SSE stream of log lines as they are written |
| `GET /runs/{id}/tasks` | Per-task execution records (for supported engines) |

## Highlights

### Submitting a Run

`POST /runs` accepts a JSON body with the engine name, workflow URL, parameters, and optional tags.
The engine and its accepted parameters are defined in `engine.yaml` — see [Engine Configuration](./engine-configuration).

### Streaming

Two SSE endpoints let you subscribe to live updates without polling:

- `GET /runs/{id}/status/stream` — emits one event per state transition, closes on terminal state
- `GET /runs/{id}/logs/stream` — emits one event per log line; use `?after_seq=N` to resume after reconnect

### Cancellation

`POST /runs/{id}/cancel` sends SIGTERM to the workflow process. Returns `409` if the run has already
reached a terminal state.

## Run States

| State | Description |
| ----- | ----------- |
| `QUEUED` | Accepted, waiting for engine |
| `INITIALIZING` | Engine setting up working directory |
| `RUNNING` | Workflow process executing |
| `COMPLETE` | Exited with code 0 |
| `EXECUTOR_ERROR` | Exited with non-zero code |
| `CANCELED` | Canceled via API |
| `SYSTEM_ERROR` | Infrastructure failure |

## Error Shape

```json
{ "msg": "Human-readable message", "status_code": 400 }
```
