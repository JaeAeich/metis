# API Reference

Metis implements the [GA4GH WES 1.1.0](https://ga4gh.github.io/workflow-execution-service-schemas/) spec with extensions for SSE streaming and log pagination.

## Base URL

```text
http://localhost:8080
```

All endpoints return JSON unless otherwise noted.

---

## Endpoints

### `GET /service-info`

Returns supported engines, WES versions, and filesystem protocols.

#### Response

```json
{
  "workflow_type_versions": { "NFL": ["DSL2"] },
  "supported_wes_versions": ["1.1.0"],
  "supported_filesystem_protocols": ["file", "http", "https"],
  "workflow_engine_versions": { "Nextflow": "25.10.4" },
  "system_state_counts": { "RUNNING": 2, "QUEUED": 1 },
  "auth_instruction_url": "",
  "tags": {}
}
```

---

### `GET /runs`

List runs with optional pagination and filtering.

#### Query Parameters

| Parameter | Type | Default | Description |
| --------- | ---- | ------- | ----------- |
| `page_size` | int | 10 | Runs per page |
| `page_token` | string | — | Cursor from previous response |
| `state` | string | — | Filter by run state |

#### Response

```json
{
  "runs": [
    { "run_id": "...", "state": "COMPLETE" }
  ],
  "next_page_token": "..."
}
```

---

### `POST /runs`

Submit a new workflow run.

#### Request Body

```json
{
  "workflow_engine": "Nextflow",
  "workflow_engine_version": "25.10.4",
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "/path/to/main.nf",
  "workflow_params": {
    "input": "/data/samples.csv",
    "outdir": "/results/output"
  },
  "workflow_engine_parameters": {
    "profile": "docker",
    "resume": "true",
    "max-cpus": "16",
    "max-memory": "64GB"
  },
  "tags": {
    "project": "my-pipeline",
    "sample": "sample-001"
  }
}
```

#### Request Fields

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `workflow_engine` | string | yes | Must match `engine.name` in a registered engine config |
| `workflow_engine_version` | string | yes | Must match `engine.version` |
| `workflow_type` | string | yes | e.g. `NFL`, `SMK`, `CWL` |
| `workflow_type_version` | string | yes | e.g. `DSL2` |
| `workflow_url` | string | yes | Path or URL to the workflow entry point |
| `workflow_params` | object | no | Workflow-level parameters |
| `workflow_engine_parameters` | object | no | Engine-level flags, validated against `engine.yaml` |
| `tags` | object | no | Arbitrary key-value metadata |

#### Response

```json
{ "run_id": "01965c3f-1234-7abc-def0-123456789abc" }
```

#### Errors

| Status | Condition |
| ------ | --------- |
| `400` | Unknown engine, unsupported workflow type, denied parameter |
| `422` | Parameter validation failed (enum, regex, range, etc.) |

---

### `GET /runs/{run_id}`

Full run log and details.

#### Response

```json
{
  "run_id": "...",
  "request": { "...": "original RunRequest fields" },
  "state": "COMPLETE",
  "run_log": {
    "name": "main.nf",
    "cmd": ["nextflow", "run", "main.nf", "-profile", "docker"],
    "start_time": "2025-01-01T00:00:00Z",
    "end_time": "2025-01-01T00:05:00Z",
    "stdout": "N E X T F L O W  ~  version 25.10.4",
    "stderr": "",
    "exit_code": 0
  },
  "task_logs": [],
  "outputs": {}
}
```

---

### `GET /runs/{run_id}/status`

Current run state only (lightweight poll).

#### Response

```json
{ "run_id": "...", "state": "RUNNING" }
```

---

### `GET /runs/{run_id}/status/stream`

SSE stream — real-time state transitions.

- Content-Type: `text/event-stream`
- Emits one event per state transition
- Closes automatically on terminal state (`COMPLETE`, `EXECUTOR_ERROR`, `CANCELED`, `SYSTEM_ERROR`)

#### Event Format

```text
data: {"run_id":"...","state":"RUNNING"}

data: {"run_id":"...","state":"COMPLETE"}
```

---

### `POST /runs/{run_id}/cancel`

Cancel a running workflow. Sends SIGTERM to the workflow process.

#### Response

```json
{ "run_id": "..." }
```

#### Errors

| Status | Condition |
| ------ | --------- |
| `404` | Run not found |
| `409` | Run already in terminal state |

---

### `DELETE /runs/{run_id}`

Soft-delete a run (sets `deleted_at`). The run record and logs are preserved in the database.

#### Response

```json
{ "run_id": "..." }
```

---

### `GET /runs/{run_id}/logs`

Paginated stdout/stderr lines.

#### Query Parameters

| Parameter | Type | Default | Description |
| --------- | ---- | ------- | ----------- |
| `stream` | string | — | Filter by `stdout` or `stderr` |
| `page_size` | int | 50 | Lines per page |
| `after_seq` | int | — | Return lines with seq > this value |

#### Response

```json
{
  "lines": [
    { "seq": 1, "stream": "stdout", "content": "N E X T F L O W  ~  version 25.10.4" },
    { "seq": 2, "stream": "stdout", "content": "[main] Launching `main.nf`" }
  ],
  "next_seq": 3
}
```

---

### `GET /runs/{run_id}/logs/stream`

SSE stream — real-time log lines as they are written.

- Content-Type: `text/event-stream`
- Emits one event per log line
- Use `?after_seq=N` to resume from a known position (safe for reconnects)
- Closes when the run reaches a terminal state and all buffered lines are flushed

#### Query Parameters

| Parameter | Type | Description |
| --------- | ---- | ----------- |
| `after_seq` | int | Start streaming from lines with seq > this value |

#### Event Format

```text
data: {"seq":1,"stream":"stdout","content":"N E X T F L O W  ~  version 25.10.4"}

data: {"seq":2,"stream":"stdout","content":"[main] Launching `main.nf`"}
```

---

### `GET /runs/{run_id}/tasks`

List task-level execution records for a run. Available for engines that report per-task data.

#### Response

```json
{
  "task_logs": [
    {
      "name": "ALIGN (sample_001)",
      "cmd": ["..."],
      "start_time": "...",
      "end_time": "...",
      "exit_code": 0,
      "stdout": "",
      "stderr": ""
    }
  ]
}
```

---

## Run States

| State | Description |
| ----- | ----------- |
| `QUEUED` | Accepted by API, waiting for engine to pick up |
| `INITIALIZING` | Engine received message, setting up working directory |
| `RUNNING` | Workflow process spawned and executing |
| `COMPLETE` | Process exited with code 0 |
| `EXECUTOR_ERROR` | Process exited with non-zero code |
| `CANCELED` | Canceled via `POST /runs/{id}/cancel` |
| `SYSTEM_ERROR` | Infrastructure failure (NATS, database, etc.) |

## Error Responses

All errors follow a consistent shape:

```json
{
  "msg": "Human-readable error message",
  "status_code": 400
}
```

| Status | Meaning |
| ------ | ------- |
| `400` | Bad request — invalid engine, workflow type, denied parameter |
| `404` | Run not found |
| `409` | Conflict — e.g. canceling a completed run |
| `422` | Unprocessable — parameter validation failed |
| `500` | Internal server error |
