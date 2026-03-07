# Deployment

## Docker Compose

The default `docker-compose.yaml` runs the full Metis stack locally.

### Services

| Service | Image | Port | Description |
| ------- | ----- | ---- | ----------- |
| `postgres` | `postgres:18-alpine` | `5432` | State and log storage |
| `valkey` | `valkey/valkey:alpine3.22` | `6379` | Engine registry and PIDs |
| `nats` | `nats:2.12.4-alpine` | `4222` / `8222` | Job queue and events |
| `metis-db-migrations` | built locally | — | Runs DB migrations on startup |
| `metis-api` | built locally | `8080` | HTTP API |
| `metis-engine-nextflow` | built locally | — | Nextflow engine runtime |
| `metis-ui` | built locally | `80` | Web UI |

### Starting

```bash
docker-compose up -d
```

Services have health checks and dependency ordering. The API and engine wait for migrations to complete before starting.

### Environment Variables

All variables have defaults. Override by setting them in the shell or a `.env` file.

| Variable | Default | Description |
| -------- | ------- | ----------- |
| `POSTGRES_DB` | `metis` | PostgreSQL database name |
| `POSTGRES_USER` | `postgres` | PostgreSQL user |
| `POSTGRES_PASSWORD` | `password` | PostgreSQL password |
| `REDIS_PASSWORD` | `password` | Valkey password |
| `NATS_USER` | `nats` | NATS username |
| `NATS_PASSWORD` | `nats` | NATS password |

The following are constructed automatically from the above:

| Variable | Value |
| -------- | ----- |
| `DATABASE_URL` | `postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB}` |
| `REDIS_URL` | `redis://:${REDIS_PASSWORD}@valkey:6379` |
| `NATS_URL` | `nats://${NATS_USER}:${NATS_PASSWORD}@nats:4222` |

### Engine Volumes

The Nextflow engine mounts several host paths:

| Mount | Description |
| ----- | ----------- |
| `./deployment/configs/engine-config.yaml` → `/etc/metis/engine-config.yaml` | Engine configuration |
| `./configs/scm` → `/root/.nextflow/scm` | Nextflow SCM credentials |
| `./workflows` → `/root/workflows` | Workflow files |
| `./configs/nextflow.config` → `/root/.nextflow/config` | Nextflow global config |
| `./configs/kubeconfig` → `/root/.kube/config` | Kubernetes config (for k8s backend) |

---

## Building Custom Images

Use `deployment/scripts/build.sh` to build all images locally:

```bash
bash deployment/scripts/build.sh
```

Individual Dockerfiles:

| Image | Dockerfile |
| ----- | ---------- |
| `metis-api` | `deployment/images/api.Dockerfile` |
| `metis-engine-nextflow` | `deployment/images/nextflow-engine.Dockerfile` |
| `metis-engine-generic` | `deployment/images/generic-engine.Dockerfile` |
| `metis-ui` | `deployment/images/ui.Dockerfile` |
| `metis-db-migrations` | `deployment/images/migration.Dockerfile` |

---

## Kubernetes

### Quickstart (Minikube)

1. Build images inside Minikube's Docker daemon:

  ```bash
  eval $(minikube docker-env)
  bash deployment/scripts/build.sh
  ```

2. Apply the service account (required for Nextflow k8s executor):

  ```bash
  kubectl apply -f configs/serviceaccount.yaml
  ```

3. Apply the quickstart manifest:

  ```bash
  kubectl apply -f deployment/k8s/quickstart.yaml
  ```

4. Access the UI:

  ```bash
  minikube service metis-ui
  ```

  Or via NodePort: `http://$(minikube ip):30080`

5. Access the API:

  ```bash
  kubectl port-forward svc/metis-api 8080:8080
  ```

### What's Included

`deployment/k8s/quickstart.yaml` deploys the full stack into the `default` namespace:

- PostgreSQL, Valkey, NATS
- DB migrations Job
- Metis API and Nextflow engine
- Metis UI (NodePort `:30080`)
- MinIO for S3-compatible object storage (NodePort `:30090`)
- Nextflow work-dir PVC (`ReadWriteMany`, 10Gi)
- ConfigMaps for engine config and Nextflow config
- Secret for SCM credentials

### Nextflow Kubernetes Backend

When running Nextflow with the `k8s` profile, Nextflow spawns worker pods directly in the cluster. This requires:

1. A `nextflow` service account with permissions to create/delete pods and access the work-dir PVC:

  ```bash
  kubectl apply -f configs/serviceaccount.yaml
  ```

2. A `ReadWriteMany` PVC for the Nextflow work directory — worker pods mount it directly.

3. The Nextflow config (embedded in the `nextflow-config` ConfigMap):

  ```groovy
  plugins {
    id 'nf-k8s'
  }

  process {
    executor = 'k8s'
  }

  k8s {
    namespace        = 'default'
    serviceAccount   = 'nextflow'
    storageClaimName = 'nextflow-work'
    storageMountPath = '/workspace'
  }

  workDir = '/workspace/work'
  ```

4. Submit runs with `"profile": "k8s"` in `workflow_engine_parameters`.

### Production Notes

The quickstart manifest uses hardcoded passwords and `emptyDir` volumes. For production:

- Replace `emptyDir` with persistent storage claims for PostgreSQL and MinIO
- Use Kubernetes Secrets for all credentials rather than plaintext in ConfigMaps
- Set resource limits on all containers
- Consider running NATS in JetStream mode for durable message delivery
- Use a proper ingress controller instead of NodePort services
