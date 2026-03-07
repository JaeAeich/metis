#!/usr/bin/env bash
# Build all Metis Docker images.
# Run from the repo root: ./deployment/scripts/build.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

IMAGES=(
  "metis-api:local:deployment/images/api.Dockerfile"
  "metis-engine-nextflow:local:deployment/images/nextflow-engine.Dockerfile"
  "metis-db-migrations:local:deployment/images/migration.Dockerfile"
  "metis-ui:local:deployment/images/ui.Dockerfile"
)

for entry in "${IMAGES[@]}"; do
  IFS=: read -r tag _ dockerfile <<<"$entry"
  image="${tag}:local"
  echo "==> Building $image from $dockerfile"
  docker build -f "$dockerfile" -t "$image" .
  echo "==> Built $image"
done

echo ""
echo "All images built:"
for entry in "${IMAGES[@]}"; do
  IFS=: read -r tag _ dockerfile <<<"$entry"
  echo "  ${tag}:local"
done
