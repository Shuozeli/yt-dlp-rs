# Design Decisions

## Why gRPC?

The gRPC architecture was chosen to allow:
- A persistent server with warm caches for faster repeated requests
- Multiple CLI clients on different platforms
- Language-agnostic API for embedding in other projects
- Bidirectional streaming for progress updates

## Why yt-dlp Subprocess?

Instead of rewriting yt-dlp's complex JS challenge-solving logic, we delegate to yt-dlp subprocess. This handles:
- Age restrictions
- Sign-in required videos
- Region blocks
- Other JS-based challenges

The project uses `--force-ipv4 --remote-components ejs:github` flags to bypass certain challenges.

## Proto-first Design

Protocol buffers define the interface between CLI and server. This ensures:
- Type-safe communication
- Language-agnostic API
- Easy client generation for other languages
- Clear contract for all features

## Download Options

`DownloadOptions` in `ytdlp-downloader` controls download behavior:

```rust
pub struct DownloadOptions {
    pub timeout: Duration,
    pub retries: u32,
    pub part_size: Option<u64>,
    pub output_template: String,
    pub proxy: String,
    pub user_agent: String,
}
```

Notably absent: `rate_limit` is not implemented because reqwest doesn't support rate limiting natively — would need a custom middleware.

## Subtitle Handling

Subtitles are handled via yt-dlp subprocess (`--list-subs`, `--write-subs`) rather than parsing YouTube's transcript API directly. This ensures:
- Consistent format support
- Auto-generated captions included
- Works across multiple sites

## Resume Support

The HTTP downloader handles resume via `Range` header when `continue_download` is set and the destination file exists. The check happens at download start time.

## Feature Discrepancies

Some proto fields are not fully wired. See `MEMORY_discrepancies.md` for details.
