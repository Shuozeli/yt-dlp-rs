# yt-dlp-rs

A Rust gRPC server and CLI for video downloading and information extraction, inspired by [yt-dlp](https://github.com/yt-dlp/yt-dlp).

**Key feature: The CLI does NOT require yt-dlp to be installed.** yt-dlp runs inside the server container, and the CLI connects to the server via gRPC.

## Architecture

```
┌──────────────┐     gRPC      ┌─────────────────┐     subprocess     ┌────────────┐
│   CLI Client │ ───────────► │  ytdlp-server   │ ─────────────────► │   yt-dlp   │
│  (any host)  │              │   (Docker)       │                    │  (inside)  │
└──────────────┘              └─────────────────┘                    └────────────┘
```

- **ytdlp-proto**: gRPC protocol definitions (protobuf)
- **ytdlp-server**: gRPC server with yt-dlp subprocess for extraction
- **ytdlp-cli**: CLI client that communicates with the server (no yt-dlp needed locally)

## Quick Start with Docker

### 1. Start the server

```bash
git clone https://github.com/Shuozeli/yt-dlp-rs.git
cd yt-dlp-rs
docker compose up -d
```

The server runs on port 50053. To change the port:

```bash
YT_DLP_SERVER_PORT=50054 docker compose up -d
```

### 2. Install the CLI

```bash
cargo install --git https://github.com/Shuozeli/yt-dlp-rs --bin ytdlp-cli
```

### 3. Use the CLI

```bash
# Set server address (or use --server flag)
export YT_DLP_SERVER=http://localhost:50053

# Extract video info
ytdlp info "https://www.youtube.com/watch?v=..."

# List formats
ytdlp info -F "https://www.youtube.com/watch?v=..."

# Download a video
ytdlp download "https://www.youtube.com/watch?v=..."

# List supported sites
ytdlp sites

# Check server health
ytdlp health
```

## Development

### Building from source

```bash
# Build all crates
cargo build --release

# Run the server
cargo run --release -p ytdlp-server

# Run the CLI (connects to localhost:50053 by default)
cargo run --release -p ytdlp-cli -- info -u "https://www.youtube.com/watch?v=..."
```

### Project structure

| Directory | Description |
|-----------|-------------|
| `ytdlp-proto/` | Protobuf definitions |
| `ytdlp-server/` | gRPC server implementation |
| `ytdlp-cli/` | CLI client |
| `ytdlp-extractor/` | Core extraction traits |
| `ytdlp-extractors/` | Site-specific extractors (YouTube, etc.) |
| `ytdlp-downloader/` | HTTP/HLS/DASH downloaders |
| `ytdlp-net/` | Networking utilities |
| `ytdlp-postproc/` | FFmpeg-based post-processing |

## Why gRPC?

The gRPC architecture provides several advantages:
- **No yt-dlp on client machines** - yt-dlp runs server-side
- **Persistent server with warm caches** - Faster extraction for repeated requests
- **Multiple clients** - CLI, web UI, or other tools can all connect to the same server
- **Language-agnostic API** - Any language with gRPC support can interact with the server
