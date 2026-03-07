#!/usr/bin/env bash
# Create a minikube cluster and deploy the full Metis stack.
# Run from the repo root: ./deployment/scripts/run-minikube.sh
#
# Prerequisites:
#   - minikube, kubectl installed
#   - Images already built: ./deployment/scripts/build.sh
#   - configs/scm contains your real GitHub PAT
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

CLUSTER_NAME="metis"
IMAGES=(
  "metis-api:local"
  "metis-engine-nextflow:local"
  "metis-db-migrations:local"
  "metis-ui:local"
)

# ---------------------------------------------------------------------------
# 1. Start / reuse minikube cluster
# ---------------------------------------------------------------------------
echo "==> Starting minikube cluster '$CLUSTER_NAME'"
if minikube status -p "$CLUSTER_NAME" --format '{{.Host}}' 2>/dev/null | grep -q "Running"; then
  echo "    Cluster already running, reusing it."
else
  minikube start -p "$CLUSTER_NAME" \
    --driver=docker \
    --cpus=4 \
    --memory=8192 \
    --mount \
    --mount-string="${HOME}:/Users/$(whoami)"
fi

# ---------------------------------------------------------------------------
# 2. Enable CSI hostpath driver for ReadWriteMany PVC support
# ---------------------------------------------------------------------------
echo ""
echo "==> Enabling CSI hostpath driver addon (required for RWX PVC)"
minikube addons enable csi-hostpath-driver -p "$CLUSTER_NAME"
minikube addons enable volumesnapshots -p "$CLUSTER_NAME"

echo "    Waiting for CSI hostpath provisioner to be ready..."
kubectl wait --for=condition=available deployment/csi-hostpathplugin \
  -n kube-system --timeout=120s 2>/dev/null ||
  kubectl rollout status daemonset/csi-hostpathplugin \
    -n kube-system --timeout=120s 2>/dev/null ||
  echo "    (CSI addon may already be ready, continuing)"

# Patch the csi-hostpath StorageClass to be the default
# (minikube's 'standard' class doesn't support RWX)
kubectl patch storageclass csi-hostpath-sc \
  -p '{"metadata":{"annotations":{"storageclass.kubernetes.io/is-default-class":"true"}}}' \
  2>/dev/null || true
kubectl patch storageclass standard \
  -p '{"metadata":{"annotations":{"storageclass.kubernetes.io/is-default-class":"false"}}}' \
  2>/dev/null || true

# ---------------------------------------------------------------------------
# 3. Load local images into minikube
# ---------------------------------------------------------------------------
echo ""
echo "==> Loading images into minikube"
for image in "${IMAGES[@]}"; do
  echo "    Loading $image"
  minikube image load "$image" -p "$CLUSTER_NAME"
done

# ---------------------------------------------------------------------------
# 4. Apply RBAC
# ---------------------------------------------------------------------------
echo ""
echo "==> Applying RBAC (serviceaccount.yaml)"
kubectl apply -f configs/serviceaccount.yaml

# ---------------------------------------------------------------------------
# 5. Create SCM secret from configs/scm
# ---------------------------------------------------------------------------
echo ""
echo "==> Creating nextflow-scm secret from configs/scm"
kubectl create secret generic nextflow-scm \
  --from-file=scm=configs/scm \
  --dry-run=client -o yaml | kubectl apply -f -

# ---------------------------------------------------------------------------
# 6. Deploy the stack
# ---------------------------------------------------------------------------
echo ""
echo "==> Applying quickstart manifests"
kubectl apply -f deployment/k8s/quickstart.yaml

# ---------------------------------------------------------------------------
# 7. Wait for core infrastructure
# ---------------------------------------------------------------------------
echo ""
echo "==> Waiting for MinIO..."
kubectl rollout status deployment/minio --timeout=120s

echo "==> Waiting for PostgreSQL..."
kubectl rollout status deployment/postgres --timeout=120s

echo "==> Waiting for Valkey..."
kubectl rollout status deployment/valkey --timeout=120s

echo "==> Waiting for NATS..."
kubectl rollout status deployment/nats --timeout=120s

echo "==> Waiting for minio-init job..."
kubectl wait --for=condition=complete job/minio-init --timeout=120s

echo "==> Waiting for metis-db-migrations job..."
kubectl wait --for=condition=complete job/metis-db-migrations --timeout=120s

# ---------------------------------------------------------------------------
# 8. Wait for app deployments
# ---------------------------------------------------------------------------
echo ""
echo "==> Waiting for metis-api..."
kubectl rollout status deployment/metis-api --timeout=180s

echo "==> Waiting for metis-engine-nextflow..."
kubectl rollout status deployment/metis-engine-nextflow --timeout=180s

echo "==> Waiting for metis-ui..."
kubectl rollout status deployment/metis-ui --timeout=120s

# ---------------------------------------------------------------------------
# 9. Verify engine config is loaded correctly
# ---------------------------------------------------------------------------
echo ""
echo "==> Verifying engine nextflow config..."
kubectl exec deployment/metis-engine-nextflow -- cat /root/.nextflow/config | grep -E "storageClaimName|workDir|storageMountPath"

# ---------------------------------------------------------------------------
# 10. Print access info
# ---------------------------------------------------------------------------
MINIKUBE_IP="$(minikube ip -p "$CLUSTER_NAME")"
echo ""
echo "========================================"
echo "  Metis stack is up!"
echo "========================================"
echo ""
echo "UI:"
echo "  http://${MINIKUBE_IP}:30080"
echo ""
echo "MinIO console:"
echo "  http://${MINIKUBE_IP}:30090  (user: root / minioroot123)"
echo ""
echo "Quick checks:"
echo "  kubectl get pods"
echo "  kubectl logs -l app=metis-engine-nextflow --tail=50"
echo "  kubectl exec deployment/metis-engine-nextflow -- cat /root/.nextflow/config"
