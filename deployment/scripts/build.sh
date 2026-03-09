#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

REGISTRY="docker.io"
NAMESPACE="jaeaeich"
TAG="latest"

PLATFORMS="linux/amd64,linux/arm64"

IMAGES=(
  "metis-api:deployment/images/api.Dockerfile"
  "metis-engine-nextflow:deployment/images/nextflow-engine.Dockerfile"
  "metis-db-migrations:deployment/images/migration.Dockerfile"
  "metis-ui:deployment/images/ui.Dockerfile"
)

echo "Using platforms: $PLATFORMS"
echo "Pushing to: $REGISTRY/$NAMESPACE"

# Ensure buildx builder exists
docker buildx create --name metis-builder --use >/dev/null 2>&1 || docker buildx use metis-builder
docker buildx inspect --bootstrap

for entry in "${IMAGES[@]}"; do
  IFS=: read -r name dockerfile <<<"$entry"

  image="$REGISTRY/$NAMESPACE/$name:$TAG"

  echo "==> Building and pushing $image"

  docker buildx build \
    --platform "$PLATFORMS" \
    -f "$dockerfile" \
    -t "$image" \
    --push \
    .

  echo "==> Pushed $image"
done

echo ""
echo "All images pushed:"
for entry in "${IMAGES[@]}"; do
  IFS=: read -r name dockerfile <<<"$entry"
  echo "  $REGISTRY/$NAMESPACE/$name:$TAG"
done
