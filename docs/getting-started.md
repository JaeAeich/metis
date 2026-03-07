# Quick Start

Get Metis running locally and submit your first workflow run in minutes.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/) v2+
- `curl` (for API calls)

## 1. Clone the Repository

```bash
git clone https://github.com/jaeaeich/metis.git
cd metis
```

## 2. Start All Services

```bash
docker-compose up -d
```

This starts six services:

| Service | Port | Description |
| ------- | ---- | ----------- |
| `metis-api` | `8080` | HTTP API |
| `metis-ui` | `80` | Web UI |
| `metis-engine-nextflow` | — | Nextflow engine runtime |
| `postgres` | `5432` | State and log storage |
| `valkey` | `6379` | Engine registry and PIDs |
| `nats` | `4222` | Job queue and events |

Wait ~15 seconds for health checks to pass, then verify:

```bash
curl http://localhost:8080/service-info
```

## 3. Submit Your First Run

Send a `POST /runs` request with workflow parameters:

```bash
curl -s -X POST http://localhost:8080/runs \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_engine": "Nextflow",
    "workflow_engine_version": "25.10.4",
    "workflow_type": "NFL",
    "workflow_type_version": "DSL2",
    "workflow_url": "/root/workflows/main.nf",
    "workflow_params": {
      "input": "/data/samples.csv",
      "outdir": "/results/output"
    },
    "workflow_engine_parameters": {
      "profile": "docker",
      "resume": "true"
    },
    "tags": {
      "project": "my-pipeline"
    }
  }'
```

The response contains the run ID:

```json
{ "run_id": "01965c3f-1234-7abc-..." }
```

## 4. Monitor the Run

### Via Web UI

Open [http://localhost](http://localhost) — the UI shows run status, logs, and state history in real time.

### Via API — Poll Status

```bash
RUN_ID="<your-run-id>"
curl http://localhost:8080/runs/$RUN_ID/status
```

```json
{ "run_id": "...", "state": "RUNNING" }
```

### Via API — Stream Status (SSE)

```bash
curl -N http://localhost:8080/runs/$RUN_ID/status/stream
```

Events are emitted on each state transition; the stream closes automatically when the run reaches a terminal state.

### Via API — Stream Logs (SSE)

```bash
curl -N http://localhost:8080/runs/$RUN_ID/logs/stream
```

Use `?after_seq=N` to resume from a known position:

```bash
curl -N "http://localhost:8080/runs/$RUN_ID/logs/stream?after_seq=42"
```

## 5. Fetch Full Run Details

```bash
curl http://localhost:8080/runs/$RUN_ID
```

Returns the complete run log including command, exit code, stdout/stderr summary, and all task logs.

## 6. Cancel a Run

```bash
curl -X POST http://localhost:8080/runs/$RUN_ID/cancel
```

## 7. Stop and Clean Up

```bash
docker-compose down
```

To also remove persisted volumes:

```bash
docker-compose down -v
```

## What's Next

- [Engine Configuration](/engine-configuration) — customise the engine, add validated parameters, set workdir layout
- [Deployment](/deployment) — production Docker Compose setup, Kubernetes, and environment variables
- [API Reference](/api-reference) — complete endpoint documentation
