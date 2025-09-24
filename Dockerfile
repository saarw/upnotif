# Build stage
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml ./
COPY src/ ./src/

RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/upnotif /upnotif

ENTRYPOINT ["/upnotif"]