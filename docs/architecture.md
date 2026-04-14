# Architecture

## System Overview

yt-dlp-rs is a Rust gRPC server + CLI for video downloading, inspired by yt-dlp.

```
┌─────────────────┐     ┌──────────────────────┐
│     ytdlp-cli   │────▶│     ytdlp-server     │
└─────────────────┘     └──────────────────────┘
                               │
                               ▼
                        ┌──────────────────────┐
                        │  yt-dlp subprocess    │
                        │ (handles JS challenges│
                        │  via --remote-components)
                        └──────────────────────┘
                               │
                               ▼ (submodule)
                        ┌──────────────────────┐
                        │  thirdparties/yt-dlp │
                        └──────────────────────┘
```

## Crates

| Crate | Purpose |
|-------|---------|
| `ytdlp-proto` | Protocol buffers and gRPC service definitions |
| `ytdlp-server` | gRPC server implementation with Tonic |
| `ytdlp-cli` | CLI client with clap |
| `ytdlp-extractors` | Video extractors (YouTube, Generic) |
| `ytdlp-extractor` | Extractor trait and common types |
| `ytdlp-downloader` | HTTP/HTTPS/HLS/DASH downloader (reqwest-based) |
| `ytdlp-net` | HTTP networking utilities |
| `ytdlp-postproc` | Post-processing: ffmpeg-based merger, embedder, subtitles converter |
| `thirdparties/yt-dlp` | yt-dlp git submodule for extraction |

## gRPC Service

The server exposes these RPCs:

| RPC | Description |
|-----|-------------|
| `Extract` | Extract video metadata (title, formats, subtitles, etc.) |
| `Download` | Download video/audio with progress streaming |
| `ListSubtitles` | List available subtitle languages |
| `DownloadSubtitles` | Download subtitles to file |
| `ListSupportedSites` | List supported extractor sites |
| `Health` | Health check |

## Data Flow

1. **CLI** parses commands and creates gRPC requests
2. **Client** sends requests to server via Tonic channel
3. **Server** finds appropriate extractor (YouTube, Generic, etc.)
4. **Extractors** use yt-dlp subprocess for JS challenge solving
5. **Downloader** handles HTTP downloads with resume support
6. **Responses** streamed back to CLI (progress events, completion)

## Key Dependencies

- **Tonic**: gRPC server and client
- **reqwest**: HTTP downloads
- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **serde**: Serialization
