# Build stage
FROM rust:1.82-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release --locked --target x86_64-unknown-linux-musl

# Runtime stage
FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/upnotif /upnotif

ENTRYPOINT ["/upnotif"]