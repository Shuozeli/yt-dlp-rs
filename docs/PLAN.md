# yt-dlp-rs Conversion Plan

## Overview

Rewrite yt-dlp core functionality in Rust with a gRPC server + CLI architecture.
Video extraction uses **yt-dlp subprocess** internally for JS challenge solving.

## Current Architecture

```
┌─────────────────┐     ┌──────────────────────┐
│     ytdlp-cli   │────▶│     ytdlp-server     │
└─────────────────┘     └──────────────────────┘
                               │
                               ▼
                        ┌──────────────────────┐
                        │  yt-dlp subprocess   │
                        │ (handles JS challenges│
                        │  via --remote-components)
                        └──────────────────────┘
                               │
                               ▼ (submodule)
                        ┌──────────────────────┐
                        │  thirdparties/yt-dlp │
                        └──────────────────────┘
```

## Project Structure

| Crate | Purpose |
|-------|---------|
| `ytdlp-proto` | Protocol buffers and gRPC service definitions |
| `ytdlp-server` | gRPC server implementation |
| `ytdlp-cli` | CLI client |
| `ytdlp-extractors` | Video extractors (YouTube, Generic) |
| `ytdlp-extractor` | Extractor trait and common types |
| `ytdlp-downloader` | HTTP/HTTPS/HLS/DASH downloader |
| `ytdlp-net` | HTTP networking utilities |
| `ytdlp-postproc` | Post-processing (ffmpeg-based) |
| `thirdparties/yt-dlp` | yt-dlp git submodule for extraction |

## Implemented Features

- [x] gRPC server + CLI architecture
- [x] Protocol buffer definitions
- [x] YouTube extractor (via yt-dlp subprocess)
- [x] Generic extractor (via yt-dlp subprocess)
- [x] Video info extraction with formats
- [x] HTTP downloader
- [x] Format URL bypass (download_url field)
- [x] Progress streaming

## Running

```bash
# Start server
cargo run --bin ytdlp-server

# In another terminal, extract info
cargo run --bin ytdlp -- info --list-formats --url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# Download video
cargo run --bin ytdlp -- download --url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' --format 18 --output video.mp4
```

## Requirements

- **yt-dlp**: Installed via `uv tool install yt-dlp`
- **Deno**: Required by yt-dlp for JS challenge solving
- **ffmpeg**: Recommended for post-processing

## TODO

- [x] HLS (m3u8) downloader
- [x] DASH manifest downloader
- [x] ffmpeg integration for format merging
- [ ] Cookie jar support
- [x] Proxy support
- [x] Subtitles extraction and conversion
