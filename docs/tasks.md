# Tasks

## Pending

### Download
- [x] HLS (m3u8) downloader
- [x] DASH manifest downloader
- [x] ffmpeg integration for format merging
- [ ] Cookie jar support

### Options
- [ ] Rate limiting in downloader (reqwest doesn't support natively)
- [ ] `continue_download` flag wired to HTTP resume logic
- [ ] `progress_interval` CLI flag

### Extraction
- [ ] `info_fields` CLI flag for partial extraction
- [ ] `extractor_opts` CLI flag for extractor-specific options

### Codecs
- [ ] Post-processing pipeline for format merging
- [ ] Subtitle format conversion (vtt → srt, etc.)

## Completed

- [x] gRPC server + CLI architecture
- [x] Protocol buffer definitions
- [x] YouTube extractor (via yt-dlp subprocess)
- [x] Generic extractor (via yt-dlp subprocess)
- [x] Video info extraction with formats
- [x] HTTP downloader with resume
- [x] Format URL bypass (download_url field)
- [x] Progress streaming
- [x] Proxy and user_agent options
- [x] ListSubtitles RPC and CLI command
- [x] DownloadSubtitles RPC and CLI command
- [x] Subtitles/thumbnails/metadata in info output
