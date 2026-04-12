# yt-dlp-rs

A Rust gRPC server and CLI for video downloading, inspired by [yt-dlp](https://github.com/yt-dlp/yt-dlp).

## Architecture

- **ytdlp-proto**: gRPC protocol definitions (protobuf)
- **ytdlp-server**: gRPC server implementing video extraction and download
- **ytdlp-cli**: CLI client for interacting with the server

## Building

```bash
cargo build --release
```

## Running the server

```bash
cargo run --release -p ytdlp-server
```

## Using the CLI

```bash
# Extract video info
cargo run --release -p ytdlp-cli -- info -u "https://www.youtube.com/watch?v=..."

# Download a video
cargo run --release -p ytdlp-cli -- download -u "https://www.youtube.com/watch?v=..."

# List supported sites
cargo run --release -p ytdlp-cli -- sites
```

## Why gRPC?

This project is a from-scratch Rust rewrite of yt-dlp's core functionality. The gRPC architecture allows:
- A persistent server with warm caches
- Multiple CLI clients on different platforms
- Language-agnostic API for embedding in other projects
