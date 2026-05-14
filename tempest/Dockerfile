FROM rust:alpine AS builder

ARG TARGETARCH

RUN apk add --no-cache musl-dev ca-certificates

# Map Buildx's TARGETARCH to the corresponding Rust target triple.
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-musl ;; \
      arm64) echo aarch64-unknown-linux-musl ;; \
      *) echo "Unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac > /rust-target.txt

ENV CLICOLOR_FORCE=1

WORKDIR /build

# Copy manifests only; Docker caches this layer until Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN RUST_TARGET=$(cat /rust-target.txt) && \
    mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release --target "$RUST_TARGET" && \
    rm -rf src

COPY src ./src
# Touch to force Cargo to relink (stub and real main share the same mtime otherwise).
RUN RUST_TARGET=$(cat /rust-target.txt) && \
    touch src/main.rs && \
    cargo build --release --target "$RUST_TARGET"

# Second pass removes .comment/.note sections that Cargo's strip leaves behind.
# Copy to a fixed path so the final stage doesn't need to know the target triple.
RUN RUST_TARGET=$(cat /rust-target.txt) && \
    strip --strip-all "/build/target/$RUST_TARGET/release/tempest" && \
    cp "/build/target/$RUST_TARGET/release/tempest" /tempest-bin

FROM scratch

# enable CLI colors by default
ENV CLICOLOR_FORCE=1

# CA bundle for reqwest/rustls HTTPS. Falls back to the embedded Mozilla bundle
# if absent (public CAs still work), but this explicit copy covers private CAs.
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

COPY --from=builder /tempest-bin /tempest

ENTRYPOINT ["/tempest"]
