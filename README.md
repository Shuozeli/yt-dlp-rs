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

## Prerequisites

- **Docker** and **Docker Compose** (for server deployment)
- **Rust 1.94+** (for building from source or CLI)
- **yt-dlp** (only needed if running server without Docker)
- **FFmpeg** (only needed if running server without Docker)

## Setup

### Option A: Docker (Recommended)

**1. Clone and start the server:**

```bash
git clone https://github.com/Shuozeli/yt-dlp-rs.git
cd yt-dlp-rs
docker compose up -d
```

**2. Install the CLI:**

```bash
cargo install --git https://github.com/Shuozeli/yt-dlp-rs --bin ytdlp-cli
```

**3. Configure CLI to connect to server:**

```bash
export YT_DLP_SERVER=http://localhost:50053
```

Or use the `--server` flag with every command:

```bash
ytdlp --server http://localhost:50053 info "https://youtube.com/watch?v=..."
```

### Option B: Local Development

**1. Install dependencies:**

```bash
# Ubuntu/Debian
sudo apt-get install yt-dlp ffmpeg protobuf-compiler

# macOS
brew install yt-dlp ffmpeg protobuf
```

**2. Clone and build:**

```bash
git clone https://github.com/Shuozeli/yt-dlp-rs.git
cd yt-dlp-rs
cargo build --release
```

**3. Run the server:**

```bash
cargo run --release -p ytdlp-server
```

**4. In another terminal, run the CLI:**

```bash
cargo run --release -p ytdlp-cli -- info "https://youtube.com/watch?v=..."
```

## Usage

### Extract video information

```bash
ytdlp info "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### List available formats

```bash
ytdlp info -F "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### Download a video

```bash
ytdlp download "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### Download with specific format

```bash
ytdlp download "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --format bestvideo+bestaudio
```

### List subtitles

```bash
ytdlp transcript "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### Download subtitles

```bash
ytdlp download-subs "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --lang en --output subtitles.vtt
```

### Other commands

```bash
ytdlp sites              # List supported sites
ytdlp health              # Check server health
ytdlp config --show-path  # Show config file location
```

## Configuration

The CLI reads from `~/.config/ytdlp-rs.toml`:

```toml
output_template = "%(title)s-%(id)s.%(ext)s"
retries = 10
rate_limit = "5M"        # Optional: e.g., "1M" for 1 MB/s
proxy = ""                # Optional: proxy URL
user_agent = ""          # Optional: custom user agent
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `YT_DLP_SERVER` | `http://127.0.0.1:50053` | Server address |
| `RUST_LOG` | `info` | Logging level |

## Project Structure

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
