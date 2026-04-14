#!/bin/bash
# Batch crawl YouTube channels for videos and transcripts

CHANNELS=(
    "https://www.youtube.com/@googledeepmind/videos"
    "https://www.youtube.com/@Insider/videos"
    "https://www.youtube.com/@MarketWatch/videos"
    "https://www.youtube.com/@TheB1M"
    "https://www.youtube.com/@TheValley101/videos"
    "https://www.youtube.com/@veritasium/videos"
    "https://www.youtube.com/@wsj/videos"
    "https://www.youtube.com/@YahooFinance/videos"
    "https://www.youtube.com/@justsayaiorg/videos"
    "https://www.youtube.com/@business/videos"
    "https://www.youtube.com/@BusinessInsider/videos"
    "https://www.youtube.com/@CNBC/videos"
    "https://www.youtube.com/@CNBCi/videos"
    "https://www.youtube.com/@companyman114/videos"
    "https://www.youtube.com/@emarketerinc/videos"
    "https://www.youtube.com/@FinancialTimes/videos"
)

OUTPUT_DIR="/tmp/channel_transcripts"
mkdir -p "$OUTPUT_DIR"

for channel_url in "${CHANNELS[@]}"; do
    echo "=========================================="
    echo "Processing: $channel_url"
    echo "=========================================="

    # Get video list from channel
    videos=$(timeout 60 yt-dlp --flat-playlist --print title,id "$channel_url" 2>/dev/null)

    if [ -z "$videos" ]; then
        echo "Failed to get videos for $channel_url"
        continue
    fi

    echo "$videos"
    echo ""

    # Process first 5 videos per channel
    count=0
    while IFS read -r line; do
        if [[ $line =~ ^[A-Za-z] ]]; then
            title="$line"
        elif [[ $line =~ ^[a-zA-Z0-9_-]{11}$ ]]; then
            video_id="$line"
            if [ -n "$title" ] && [ -n "$video_id" ]; then
                echo "  -> $title ($video_id)"

                # Check if transcript already exists
                if [ -f "$OUTPUT_DIR/${video_id}.en.vtt" ]; then
                    echo "     [SKIP] Transcript exists"
                else
                    # Try to download transcript
                    cargo run --bin ytdlp --quiet -- download-subs \
                        --url "https://www.youtube.com/watch?v=$video_id" \
                        --lang en \
                        --output "$OUTPUT_DIR/" 2>/dev/null

                    if [ -f "$OUTPUT_DIR/"*.${video_id}.en.vtt ]; then
                        echo "     [OK] Transcript downloaded"
                    else
                        echo "     [SKIP] No transcript available"
                    fi
                fi

                title=""
                video_id=""
                count=$((count + 1))

                if [ $count -ge 5 ]; then
                    break
                fi
            fi
        fi
    done <<< "$videos"

    echo ""
done

echo "=========================================="
echo "Crawl complete. Transcripts saved to: $OUTPUT_DIR"
echo "=========================================="
ls -la "$OUTPUT_DIR/" | head -20
