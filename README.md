# UpNotif - URL Uptime Monitor

A lightweight Rust application that monitors URL uptime and sends Slack notifications when status changes occur.

## Features

- Monitors multiple URLs for 2xx HTTP responses
- Uses rustls for maximum platform independence
- Sends Slack webhook notifications on status changes
- Reports initial status on startup
- Configurable check intervals
- Built with musl for static linking

## Environment Variables

- `UPNOTIF_URLS` - Comma-separated list of URLs to monitor (required)
- `UPNOTIF_SLACK_WEBHOOK` - Slack webhook URL for notifications, or "test" for console output (required)
- `UPNOTIF_INTERVAL_SECONDS` - Check interval in seconds (optional, defaults to 60)

### Test Mode

Set `UPNOTIF_SLACK_WEBHOOK=test` to run in test mode. Instead of sending notifications to Slack, all messages will be logged to the console. This is useful for:
- Testing the application before deploying
- Development and debugging
- Running without a Slack webhook

## Usage

### Production:
```bash
export UPNOTIF_URLS="https://example.com,https://google.com"
export UPNOTIF_SLACK_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
export UPNOTIF_INTERVAL_SECONDS=30

./upnotif
```

### Test mode:
```bash
export UPNOTIF_URLS="https://example.com,https://google.com"
export UPNOTIF_SLACK_WEBHOOK="test"
export UPNOTIF_INTERVAL_SECONDS=10

./upnotif
# Output will be logged to console instead of sent to Slack
# The program logs at INFO level by default - no need to set RUST_LOG
```

## Building

### For current platform (Mac/Windows/Linux):
```bash
cargo build --release
```
Binary will be in `target/release/upnotif` (or `upnotif.exe` on Windows).

### For Linux musl (static binary):
```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl
```
Binary will be in `target/x86_64-unknown-linux-musl/release/upnotif` and can run on any Linux system without dependencies.

### Platform-specific notes:
- **Mac**: Use the default build - it will create a native binary with system dependencies
- **Linux**: Use musl target for maximum portability
- **Windows**: Default build works fine

## Docker

### Building and running with Docker:
```bash
# Build the Docker image
docker build -t upnotif .

# Run the container
docker run --rm \
  -e UPNOTIF_URLS="https://example.com,https://google.com" \
  -e UPNOTIF_SLACK_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL" \
  -e UPNOTIF_INTERVAL_SECONDS=30 \
  upnotif
```

### Using Docker Compose:
```bash
# Edit docker-compose.yml with your environment variables
docker-compose up -d
```

The multi-stage Dockerfile:
1. **Build stage**: Uses Rust Alpine image to compile the application with musl
2. **Runtime stage**: Uses `scratch` base (minimal possible image) with only the static binary
3. Final image is extremely small (~10MB) and runs anywhere