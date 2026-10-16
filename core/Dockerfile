# Rust gRPC-сервер CodeGraph.
FROM rust:1.83-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
        protobuf-compiler \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY ir/Cargo.toml ir/
COPY parser/Cargo.toml parser/
COPY graph/Cargo.toml graph/
COPY grpc/Cargo.toml grpc/

RUN mkdir -p ir/src parser/src graph/src grpc/src \
    && echo "fn main() {}" > grpc/src/main.rs \
    && echo "" > ir/src/lib.rs \
    && echo "" > parser/src/lib.rs \
    && echo "" > graph/src/lib.rs \
    && echo "" > grpc/src/lib.rs

RUN cargo build --release -p codegraph-grpc 2>/dev/null || true
COPY . .
RUN cargo build --release -p codegraph-grpc
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/codegraph-core /usr/local/bin/codegraph-core

EXPOSE 50051
CMD ["codegraph-core"]