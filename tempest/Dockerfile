FROM rust:1 AS builder
WORKDIR /app
# Cache dependency compilation separately from application code.
# Copy manifests first, build a dummy main, then copy src.
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --bin tempest

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -m tempest
USER tempest
WORKDIR /app
COPY --from=builder /app/target/release/tempest /usr/local/bin/tempest
CMD ["tempest", "test", "--path", "/etc/tests"]