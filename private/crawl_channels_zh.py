#!/usr/bin/env python3
"""Batch crawl YouTube channels for Chinese transcripts."""

import subprocess
import time
from pathlib import Path

# Non-English channels that need Chinese
CHANNELS_ZH = [
    ("thevalley101", "https://www.youtube.com/@TheValley101/videos"),
    ("justsayai", "https://www.youtube.com/@justsayaiorg/videos"),
    ("emarketer", "https://www.youtube.com/@emarketerinc/videos"),
]

OUTPUT_DIR = Path("/tmp/channel_transcripts_zh")
OUTPUT_DIR.mkdir(exist_ok=True)

YTDLP_FLAGS = ["--force-ipv4", "--remote-components", "ejs:github", "--flat-playlist", "--playlist-end", "5"]


def get_channel_videos(channel_url: str) -> list[tuple[str, str]]:
    cmd = [
        "yt-dlp",
        *YTDLP_FLAGS,
        "--print", "title,id",
        channel_url,
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        if result.returncode != 0:
            return []

        videos = []
        lines = result.stdout.strip().split("\n")
        i = 0
        while i < len(lines) - 1:
            title = lines[i].strip()
            video_id = lines[i + 1].strip()
            if title and video_id and len(video_id) == 11:
                videos.append((title, video_id))
                i += 2
            else:
                i += 1
        return videos
    except Exception:
        return []


def download_transcript(video_id: str, lang: str) -> bool:
    output_path = OUTPUT_DIR / f"{video_id}.{lang}.vtt"
    if output_path.exists():
        print(f"    [SKIP] Already exists")
        return True

    cmd = [
        "cargo", "run", "--bin", "ytdlp", "--",
        "download-subs",
        "--url", f"https://www.youtube.com/watch?v={video_id}",
        "--lang", lang,
        "--output", str(OUTPUT_DIR) + "/",
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        return result.returncode == 0
    except Exception:
        return False


for name, url in CHANNELS_ZH:
    print(f"\n{'='*60}")
    print(f"Channel: {name} ({url})")
    print(f"{'='*60}")

    videos = get_channel_videos(url)
    if not videos:
        print("  [WARN] No videos found")
        continue

    print(f"  Found {len(videos)} videos")

    for title, video_id in videos:
        print(f"  - {title[:50]}... ({video_id})")

        # Try Chinese first
        if download_transcript(video_id, "zh"):
            print(f"    [OK] Chinese transcript")
        elif download_transcript(video_id, "zh-Hans"):
            print(f"    [OK] Chinese (Simplified) transcript")
        else:
            print(f"    [SKIP] No Chinese transcript available")

        time.sleep(0.5)

    time.sleep(1)

# Summary
print(f"\n{'='*60}")
print("CRAWL COMPLETE")
print(f"{'='*60}")

vtt_files = list(OUTPUT_DIR.glob("*.vtt"))
print(f"Total Chinese transcripts: {len(vtt_files)}")

for f in sorted(vtt_files)[:10]:
    print(f"  {f.name} ({f.stat().st_size} bytes)")
