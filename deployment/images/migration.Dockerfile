# syntax=docker/dockerfile:1.4
# =============================================================================
# Metis Database Migrations
# =============================================================================

ARG MIGRATE_VERSION=v4.17.0

FROM migrate/migrate:${MIGRATE_VERSION} AS runtime

COPY --chown=nobody:nobody migrations/ /migrations

USER nobody:nobody
