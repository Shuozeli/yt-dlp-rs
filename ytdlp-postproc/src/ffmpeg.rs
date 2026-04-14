//! FFmpeg wrapper for yt-dlp-rs
//!
//! Provides detection, execution, and probing capabilities for ffmpeg/ffprobe.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

/// FFmpeg instance wrapping an external ffmpeg binary
pub struct Ffmpeg {
    path: PathBuf,
    version: String,
}

impl Ffmpeg {
    /// Detect ffmpeg by searching PATH for "ffmpeg"
    pub fn detect() -> Option<Self> {
        Self::from_path(Path::new("ffmpeg"))
    }

    /// Create an Ffmpeg instance from a specific path
    pub fn from_path(path: &Path) -> Option<Self> {
        let output = std::process::Command::new(path)
            .arg("-version")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()?
            .to_string();

        Some(Self {
            path: path.to_path_buf(),
            version,
        })
    }

    /// Run ffmpeg with the given arguments
    pub async fn run(&self, args: &[&str]) -> Result<Output> {
        let output = tokio::process::Command::new(&self.path)
            .args(args)
            .output()
            .await
            .with_context(|| format!("failed to execute ffmpeg: {}", self.path.display()))?;

        Ok(output)
    }

    /// Run ffmpeg and return an error if it fails
    pub async fn run_checked(&self, args: &[&str]) -> Result<()> {
        let output = self.run(args).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ffmpeg failed: {}", stderr);
        }

        Ok(())
    }

    /// Probe media file using ffprobe to get media information
    pub async fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let output = tokio::process::Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
            .await
            .context("failed to execute ffprobe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ffprobe failed: {}", stderr);
        }

        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("failed to parse ffprobe output")?;

        // Parse duration from format section
        let duration = if let Some(dur_val) = json.get("format").and_then(|f| f.get("duration")) {
            dur_val
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .map(Duration::from_secs_f64)
        } else {
            None
        };

        // Parse streams from streams array
        let streams: Vec<StreamInfo> =
            if let Some(streams_arr) = json.get("streams").and_then(|s| s.as_array()) {
                streams_arr
                    .iter()
                    .filter_map(|s| {
                        let codec = s.get("codec_name")?.as_str()?.to_string();
                        let bitrate = s
                            .get("bit_rate")
                            .and_then(|b| b.as_str())
                            .and_then(|b| b.parse::<u64>().ok());
                        let width = s.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
                        let height = s.get("height").and_then(|h| h.as_u64()).map(|h| h as u32);

                        Some(StreamInfo {
                            codec,
                            bitrate,
                            width,
                            height,
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            };

        Ok(MediaInfo { duration, streams })
    }

    /// Get the ffmpeg version string
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the path to the ffmpeg binary
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Media information probed from a file
#[derive(Debug, Clone)]
pub struct MediaInfo {
    /// Duration of the media
    pub duration: Option<Duration>,
    /// Stream information
    pub streams: Vec<StreamInfo>,
}

/// Information about a single media stream
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// Codec name (e.g., "h264", "aac")
    pub codec: String,
    /// Bitrate in bits per second
    pub bitrate: Option<u64>,
    /// Video width in pixels (None for audio streams)
    pub width: Option<u32>,
    /// Video height in pixels (None for audio streams)
    pub height: Option<u32>,
}
