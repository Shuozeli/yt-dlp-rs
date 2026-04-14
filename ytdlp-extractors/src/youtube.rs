//! YouTube extractor for yt-dlp-rs
//!
//! Extracts video information from YouTube URLs.
//! Uses yt-dlp subprocess to handle signature decryption and format extraction.

use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use ytdlp_extractor::{Extractor, Format, VideoInfo};

/// YouTube video extractor using yt-dlp subprocess
#[derive(Default)]
pub struct YoutubeExtractor;

impl YoutubeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract video ID from various YouTube URL formats
    pub fn extract_video_id(url: &str) -> Result<String> {
        let patterns = [
            r"youtube\.com/watch\?v=([^&]+)",
            r"youtu\.be/([^?]+)",
            r"youtube\.com/embed/([^?]+)",
            r"youtube\.com/v/([^?]+)",
            r"youtube\.com/shorts/([^?]+)",
            r"youtube\.com/live/([^?]+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(url) {
                    return Ok(caps[1].to_string());
                }
            }
        }
        anyhow::bail!("Could not extract video ID from URL: {}", url)
    }

    /// Run yt-dlp to extract video info as JSON
    async fn run_ytdlp(&self, url: &str) -> Result<YtDlpOutput> {
        // Use yt-dlp directly (installed via uv tool) with IPv4 and remote components
        let mut child = Command::new("yt-dlp")
            .args([
                "--dump-json",
                "--no-warnings",
                "--force-ipv4",
                "--remote-components",
                "ejs:github",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to spawn yt-dlp process")?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        {
            let stdout_pipe = child.stdout.take();
            let stderr_pipe = child.stderr.take();

            if let Some(mut s) = stdout_pipe {
                s.read_to_string(&mut stdout)
                    .await
                    .context("Failed to read stdout")?;
            }
            if let Some(mut s) = stderr_pipe {
                s.read_to_string(&mut stderr)
                    .await
                    .context("Failed to read stderr")?;
            }
        }

        let status = child.wait().await.context("Failed to wait for yt-dlp")?;

        if !status.success() {
            anyhow::bail!("yt-dlp failed: {}", stderr);
        }

        // Parse the JSON output
        let output: YtDlpOutput =
            serde_json::from_str(&stdout).context("Failed to parse yt-dlp JSON output")?;

        Ok(output)
    }
}

/// yt-dlp JSON output structure
#[derive(Debug, Deserialize)]
pub struct YtDlpOutput {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub uploader_url: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub formats: Vec<YtDlpFormat>,
}

#[derive(Debug, Deserialize)]
pub struct YtDlpFormat {
    pub format_id: String,
    pub ext: String,
    pub url: String,
    pub filesize: Option<u64>,
    pub format: Option<String>,
    pub format_note: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub fps: Option<f64>,
    pub tbr: Option<f64>,
    pub resolution: Option<String>,
    #[serde(alias = "container")]
    pub container: Option<String>,
    #[serde(alias = "video_ext")]
    pub video_ext: Option<String>,
    #[serde(alias = "audio_ext")]
    pub audio_ext: Option<String>,
}

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn name(&self) -> &str {
        "youtube"
    }

    fn supported_domains(&self) -> &[&str] {
        &[
            "youtube.com",
            "youtu.be",
            "www.youtube.com",
            "m.youtube.com",
        ]
    }

    async fn extract(&self, url: &str) -> Result<VideoInfo> {
        let video_id = Self::extract_video_id(url)?;

        tracing::info!("Extracting YouTube video info via yt-dlp: {}", video_id);

        // Use yt-dlp to get video info
        let output = self.run_ytdlp(url).await?;

        // Convert yt-dlp formats to our format
        let formats: Vec<Format> = output
            .formats
            .into_iter()
            .filter(|f| !f.url.is_empty())
            .map(|f| {
                let resolution = f.resolution.or(f.format_note);
                let tbr = f.tbr.map(|t| t / 1000.0); // Convert to kbps

                Format {
                    format_id: f.format_id,
                    ext: f.ext,
                    resolution,
                    filesize: f.filesize,
                    vcodec: f.vcodec.filter(|c| c != "none"),
                    acodec: f.acodec.filter(|c| c != "none"),
                    fps: f.fps.map(|f| f as u32),
                    tbr,
                    protocol: "https".to_string(),
                    url: Some(f.url),
                }
            })
            .collect();

        // Build thumbnail URL if not provided
        let thumbnail = output.thumbnail.or_else(|| {
            Some(format!(
                "https://i.ytimg.com/vi/{}/maxresdefault.jpg",
                video_id
            ))
        });

        let duration = output
            .duration
            .map(|d| std::time::Duration::from_secs(d as u64));

        Ok(VideoInfo {
            id: output.id,
            title: output.title,
            description: output.description,
            uploader: output.uploader,
            uploader_url: output.uploader_url,
            duration,
            thumbnail: thumbnail.and_then(|t| url::Url::parse(&t).ok()),
            formats,
            subtitles: HashMap::new(),
            thumbnails: vec![],
            metadata: HashMap::new(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        let test_cases = vec![
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/embed/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/shorts/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
        ];

        for (url, expected_id) in test_cases {
            let result = YoutubeExtractor::extract_video_id(url);
            assert!(result.is_ok(), "Failed for URL: {}", url);
            assert_eq!(result.unwrap(), expected_id, "Mismatch for URL: {}", url);
        }
    }
}
