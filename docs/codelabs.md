# Codelabs

## Quick Start

### Build

```bash
cargo build --release
```

### Start Server

```bash
cargo run --bin ytdlp-server
```

The server runs on `http://127.0.0.1:50053` by default.

## CLI Commands

### Extract Video Info

```bash
# Basic info
cargo run --bin ytdlp -- info --url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# List available formats
cargo run --bin ytdlp -- info -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' -F

# JSON output
cargo run --bin ytdlp -- info -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' --json
```

### Download Video

```bash
# Download with best format
cargo run --bin ytdlp -- download --url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# Download specific format
cargo run --bin ytdlp -- download -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' --format 18

# With options
cargo run --bin ytdlp -- download \
  -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' \
  --format best \
  --retries 5 \
  --proxy http://proxy:8080 \
  --user-agent "Mozilla/5.0" \
  --continue
```

### List Subtitles

```bash
# List available subtitles for a video
cargo run --bin ytdlp -- transcript --url 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# Filter by language
cargo run --bin ytdlp -- transcript -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' --lang en
```

### Download Subtitles

```bash
# Download English subtitles
cargo run --bin ytdlp -- download-subs \
  -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' \
  --lang en \
  --output /tmp/subs/

# Download with specific format (vtt, srt, ttml, json3)
cargo run --bin ytdlp -- download-subs \
  -u 'https://www.youtube.com/watch?v=dQw4w9WgXcQ' \
  --lang en \
  --format srt \
  --output /tmp/subs/
```

### Other Commands

```bash
# List supported sites
cargo run --bin ytdlp -- sites

# Search sites
cargo run --bin ytdlp -- sites --search youtube

# Health check
cargo run --bin ytdlp -- health

# Show config
cargo run --bin ytdlp -- config
```

## Using with Custom Server

```bash
# Connect to different server
cargo run --bin ytdlp -- --server http://192.168.1.100:50053 info -u 'https://...'
```

## Channel Videos

To list videos from a YouTube channel:

```bash
yt-dlp --flat-playlist --playlist-end 5 --print title,id 'https://www.youtube.com/@CNBC'
```

Then use the video URLs with the transcript commands above.
