ARG RUST_VERSION=1.93.1
FROM rust:${RUST_VERSION}-alpine AS base
RUN apk add --no-cache musl-dev curl
RUN cargo install cargo-chef
WORKDIR /app
