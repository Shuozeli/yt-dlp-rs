#!/usr/bin/env python3
"""Batch crawl YouTube channels for transcripts."""

import asyncio
import os
import subprocess
import sys
import time
from pathlib import Path

# Channel list from kbdriverv4 sources.yaml
CHANNELS = [
    ("google_deepmind", "https://www.youtube.com/@googledeepmind/videos"),
    ("insider", "https://www.youtube.com/@Insider/videos"),
    ("marketwatch", "https://www.youtube.com/@MarketWatch/videos"),
    ("theb1m", "https://www.youtube.com/@TheB1M"),
    ("thevalley101", "https://www.youtube.com/@TheValley101/videos"),
    ("veritasium", "https://www.youtube.com/@veritasium/videos"),
    ("wsj", "https://www.youtube.com/@wsj/videos"),
    ("yahoofinance", "https://www.youtube.com/@YahooFinance/videos"),
    ("justsayai", "https://www.youtube.com/@justsayaiorg/videos"),
    ("bloomberg", "https://www.youtube.com/@business/videos"),
    ("businessinsider", "https://www.youtube.com/@BusinessInsider/videos"),
    ("cnbc", "https://www.youtube.com/@CNBC/videos"),
    ("cnbci", "https://www.youtube.com/@CNBCi/videos"),
    ("companyman", "https://www.youtube.com/@companyman114/videos"),
    ("emarketer", "https://www.youtube.com/@emarketerinc/videos"),
    ("ft", "https://www.youtube.com/@FinancialTimes/videos"),
]

OUTPUT_DIR = Path("/tmp/channel_transcripts")
OUTPUT_DIR.mkdir(exist_ok=True)

YTDLP_FLAGS = ["--force-ipv4", "--remote-components", "ejs:github", "--flat-playlist", "--playlist-end", "5"]


def get_channel_videos(channel_url: str) -> list[tuple[str, str]]:
    """Get video titles and IDs from a channel using yt-dlp."""
    cmd = [
        "yt-dlp",
        *YTDLP_FLAGS,
        "--print", "title,id",
        channel_url,
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        if result.returncode != 0:
            print(f"  [WARN] yt-dlp failed: {result.stderr[:200]}")
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
    except Exception as e:
        print(f"  [ERROR] {e}")
        return []


def download_transcript(video_id: str, title: str) -> bool:
    """Download transcript for a video using our ytdlp CLI."""
    output_path = OUTPUT_DIR / f"{video_id}.en.vtt"

    # Check if already downloaded
    if output_path.exists():
        print(f"    [SKIP] Already exists")
        return True

    # Build ytdlp CLI command
    cmd = [
        "cargo", "run", "--bin", "ytdlp", "--",
        "download-subs",
        "--url", f"https://www.youtube.com/watch?v={video_id}",
        "--lang", "en",
        "--output", str(OUTPUT_DIR) + "/",
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        if result.returncode == 0:
            print(f"    [OK] Downloaded")
            return True
        else:
            print(f"    [WARN] {result.stderr[:100] if result.stderr else 'failed'}")
            return False
    except Exception as e:
        print(f"    [ERROR] {e}")
        return False


async def process_channel(name: str, url: str):
    """Process a single channel."""
    print(f"\n{'='*60}")
    print(f"Channel: {name}")
    print(f"URL: {url}")
    print(f"{'='*60}")

    videos = get_channel_videos(url)
    if not videos:
        print("  [WARN] No videos found")
        return

    print(f"  Found {len(videos)} videos")

    for i, (title, video_id) in enumerate(videos, 1):
        print(f"  [{i}] {title[:50]}... ({video_id})")
        download_transcript(video_id, title)
        time.sleep(0.5)  # Rate limit


async def main():
    print(f"Starting crawl. Output dir: {OUTPUT_DIR}")
    print(f"Processing {len(CHANNELS)} channels")
    print(f"Max 5 videos per channel")

    for name, url in CHANNELS:
        await process_channel(name, url)
        time.sleep(1)  # Delay between channels

    # Summary
    print(f"\n{'='*60}")
    print("CRAWL COMPLETE")
    print(f"{'='*60}")

    vtt_files = list(OUTPUT_DIR.glob("*.vtt"))
    print(f"Total transcripts: {len(vtt_files)}")

    # Show latest
    if vtt_files:
        print("\nRecent transcripts:")
        for f in sorted(vtt_files, key=lambda x: x.stat().st_mtime, reverse=True)[:10]:
            size = f.stat().st_size
            print(f"  {f.name} ({size} bytes)")


if __name__ == "__main__":
    asyncio.run(main())
