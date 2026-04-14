# yt-dlp-rs Project Skill

## What This Project Is
yt-dlp-rs is a gRPC server + CLI wrapper around yt-dlp. The CLI does NOT require yt-dlp installed locally - yt-dlp runs inside the Docker container on the server.

## Quick Start

**1. Start the server:**
```bash
docker compose up -d
```

**2. Install CLI:**
```bash
cargo install --git https://github.com/Shuozeli/yt-dlp-rs --bin ytdlp-cli
```

**3. Use CLI:**
```bash
export YT_DLP_SERVER=http://localhost:50053
ytdlp info "https://youtube.com/watch?v=..."
```

## Key Commands

| Command | Description |
|---------|-------------|
| `ytdlp info <url>` | Extract video info |
| `ytdlp info -F <url>` | List available formats |
| `ytdlp download <url>` | Download video |
| `ytdlp sites` | List supported sites |
| `ytdlp health` | Check server health |

## Architecture
- Server: gRPC on port 50053 (Docker)
- CLI: Connects to server via gRPC (no yt-dlp needed on client)
- yt-dlp runs as subprocess inside the server container

## Important Notes
- The CLI only sends gRPC requests to the server - it never calls yt-dlp directly
- yt-dlp binary, ffmpeg, and all dependencies are inside the Docker container
- Server needs network access to YouTube and other video sites
