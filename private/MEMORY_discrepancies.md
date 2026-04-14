# yt-dlp-rs Feature Discrepancies

## Fully Wired (no issues)
- `ListSubtitles` / `transcript` ✅
- `DownloadSubtitles` / `download-subs` ✅
- `ListSupportedSites` / `sites` ✅
- `Health` / `health` ✅

## Partially Wired Issues

### 1. Download Options Not Passed to Downloader
Proto adds `rate_limit`, `proxy`, `user_agent`, `continue_download` to `DownloadRequest`, CLI accepts flags, client passes them — but service ignores them.

**Location**: `ytdlp-server/src/service.rs` download method builds `DownloadOptions` only from `retries`:
```rust
let options = DownloadOptions {
    retries: if req.retries > 0 { req.retries as u32 } else { 3 },
    ..Default::default()
};
```

**Fix needed**: `ytdlp-downloader/src/http.rs` needs to accept these options and apply them to the HTTP client.

### 2. Extract `info_fields` and `extractor_opts` Not Exposed
Proto defines these fields but CLI has no flags and client hardcodes empty/defaults.

### 3. `info_cmd` Missing Display Fields
CLI `info_cmd` doesn't show `subtitles`, `thumbnails`, or `metadata` from `VideoInfo` — even in JSON mode.

### 4. Playlist Flags Unused
`--playlist-start` and `--playlist-end` are underscore-prefixed in `download_cmd` — not passed anywhere.

### 5. `progress_interval` Not Exposed
Proto field exists but CLI has no flag and client hardcodes 0.

## Files
- Proto: `ytdlp-proto/proto/ytdlp.proto`
- Service: `ytdlp-server/src/service.rs`
- CLI main: `ytdlp-cli/src/main.rs`
- Client: `ytdlp-cli/src/client.rs`
- HTTP downloader: `ytdlp-downloader/src/http.rs`

---

## Updates (2026-04-13)

### Fixed
- **Playlist flags removed**: `--playlist-start` and `--playlist-end` were removed from CLI since they weren't wired
- **`info_cmd` now shows subtitles/thumbnails/metadata**: Added to both JSON and text output
- **Download options wired (proxy, user_agent)**: `DownloadOptions` now includes `proxy` and `user_agent` fields; `HttpDownloader::with_options()` applies them; service passes them from proto request

### Still Missing
- **`rate_limit`**: Not implemented — would need custom rate limiting (reqwest doesn't support it natively)
- **`continue_download`**: The Range header resume logic exists in http.rs but isn't controlled by this flag
- **`progress_interval`**: CLI has no flag, client hardcodes 0
- **`info_fields` / `extractor_opts`**: Not exposed in CLI
