ARG RUST_VERSION=1.93.1

FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache musl-dev curl
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
RUN cargo build --release --bin metis-api

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/metis-api /usr/local/bin/
EXPOSE 8080
CMD ["metis-api"]
