FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev ca-certificates

ENV CLICOLOR_FORCE=1

WORKDIR /build

# Copy manifests only; Docker caches this layer until Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

COPY src ./src
# Touch to force Cargo to relink (stub and real main share the same mtime otherwise).
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl

# Second pass removes .comment/.note sections that Cargo's strip leaves behind.
RUN strip --strip-all /build/target/x86_64-unknown-linux-musl/release/tempest

FROM scratch

# enable CLI colors by default
ENV CLICOLOR_FORCE=1

# CA bundle for reqwest/rustls HTTPS. Falls back to the embedded Mozilla bundle
# if absent (public CAs still work), but this explicit copy covers private CAs.
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/tempest /tempest

ENTRYPOINT ["/tempest"]
