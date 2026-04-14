//! Subtitles converter and embedder for yt-dlp-rs
//!
//! Provides functionality to convert subtitles between formats
//! and embed them into video files.

use crate::ffmpeg::Ffmpeg;
use anyhow::{Context, Result};
use std::path::Path;

/// Converter for subtitle formats
pub struct SubtitlesConverter;

impl SubtitlesConverter {
    /// Convert subtitles from one format to another
    ///
    /// # Arguments
    /// * `input` - Path to the input subtitle file
    /// * `from_format` - Source format (e.g., "srt", "vtt")
    /// * `to_format` - Target format (e.g., "srt", "vtt", "ass")
    /// * `output` - Path for the converted subtitle file
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn convert(
        &self,
        input: &Path,
        from_format: &str,
        to_format: &str,
        output: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        let codec = match to_format {
            "ass" | "ssa" => "ass",
            "srt" => "srt",
            "vtt" => "webvtt",
            _ => to_format,
        };

        ffmpeg
            .run_checked(&[
                "-i",
                input.to_str().unwrap_or_default(),
                "-c:s",
                codec,
                "-y",
                output.to_str().unwrap_or_default(),
            ])
            .await
            .with_context(|| {
                format!(
                    "failed to convert subtitles from {} to {}",
                    from_format, to_format
                )
            })?;

        Ok(())
    }

    /// Embed subtitles into video as a new subtitle stream
    ///
    /// # Arguments
    /// * `video_path` - Path to the input video file
    /// * `subtitle_path` - Path to the subtitle file
    /// * `language` - ISO 639-1 language code (e.g., "en", "ja")
    /// * `output_path` - Path for the output file with embedded subtitles
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn embed(
        &self,
        video_path: &Path,
        subtitle_path: &Path,
        language: &str,
        output_path: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        // Determine the subtitle codec based on file extension
        let codec = subtitle_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| match e.to_lowercase().as_str() {
                "ass" | "ssa" => "ass",
                "srt" => "srt",
                "vtt" => "webvtt",
                _ => "srt",
            })
            .unwrap_or("srt");

        ffmpeg
            .run_checked(&[
                "-i",
                video_path.to_str().unwrap_or_default(),
                "-i",
                subtitle_path.to_str().unwrap_or_default(),
                "-c:v",
                "copy",
                "-c:a",
                "copy",
                "-c:s",
                codec,
                "-metadata:s:s:0",
                &format!("language={}", language),
                "-y",
                output_path.to_str().unwrap_or_default(),
            ])
            .await
            .with_context(|| {
                format!(
                    "failed to embed subtitles from {} into {}",
                    subtitle_path.display(),
                    video_path.display()
                )
            })?;

        Ok(())
    }
}
