# 1. Build stage
FROM golang:1.24.4 AS base
ARG VCS_REF
ARG VERSION
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -ldflags="-X 'github.com/jaeaeich/metis/internal/config.Version=${VERSION}' -X 'github.com/jaeaeich/metis/internal/config.GitCommit=${VCS_REF}'" -o /go/bin/metis ./cmd

# 2. Distroless image (default)
FROM gcr.io/distroless/static-debian12 AS distroless
ARG VCS_REF
ARG VERSION
LABEL org.opencontainers.image.title="metis" \
      org.opencontainers.image.description="Metis is a Workflow Execution Service (WES) API implementation." \
      org.opencontainers.image.source="https://github.com/jaeaeich/metis" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.authors="Javed Habib" \
      org.opencontainers.image.vendor="Javed Habib" \
      org.opencontainers.image.url="https://github.com/jaeaeich/metis" \
      org.opencontainers.image.documentation="https://github.com/jaeaeich/metis" \
      org.opencontainers.image.contact="jae.eichhorn@gmail.com"
COPY --from=base /go/bin/metis /
COPY --from=base /src/public /public
HEALTHCHECK --interval=30s --timeout=3s \
  CMD ["/metis", "healthz"]

# 3. Alpine image
FROM alpine:latest AS alpine
ARG APP_USER=appuser
ARG APP_GROUP=appgroup
ARG VCS_REF
ARG VERSION
LABEL org.opencontainers.image.title="metis" \
      org.opencontainers.image.description="Metis is a Workflow Execution Service (WES) API implementation." \
      org.opencontainers.image.source="https://github.com/jaeaeich/metis" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.authors="Javed Habib" \
      org.opencontainers.image.vendor="Javed Habib" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.version="${VERSION}"
RUN addgroup -S ${APP_GROUP} && adduser -S ${APP_USER} -G ${APP_GROUP}
USER ${APP_USER}:${APP_GROUP}
RUN apk add --no-cache ca-certificates
COPY --from=base /go/bin/metis /
COPY --from=base /src/public /public
HEALTHCHECK --interval=30s --timeout=3s \
  CMD ["/metis", "healthz"]

# 4. Dev image
FROM golang:1.24.4 AS dev
RUN apt-get update && apt-get install -y vim curl
WORKDIR /src
COPY --from=base /src .
CMD ["air"]
