ARG RUST_VERSION=1.93.1

FROM rust:${RUST_VERSION} AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY .sqlx ./.sqlx
RUN cargo build --release --bin metis-engine-generic

FROM eclipse-temurin:21-jre-noble
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && \
    curl -s https://get.nextflow.io | bash && \
    mv nextflow /usr/local/bin/nextflow && \
    nextflow -version && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/metis-engine-generic /usr/local/bin/
EXPOSE 8080
CMD ["metis-engine-generic", "server", "--engine-config", "/etc/metis/engine-config.yaml"]
