//! Video and audio merger for yt-dlp-rs
//!
//! Provides functionality to merge separate video and audio files
//! into a single container, as well as merging fragmented DASH/HLS downloads.

use crate::ffmpeg::Ffmpeg;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Merger for combining video and audio streams
pub struct Merger;

impl Merger {
    /// Merge video file and audio file into a single output file
    ///
    /// Uses ffmpeg to combine the video stream from `video_path` with
    /// the audio stream from `audio_path` into `output_path`.
    ///
    /// # Arguments
    /// * `video_path` - Path to the input video file
    /// * `audio_path` - Path to the input audio file
    /// * `output_path` - Path for the merged output file
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn merge(
        &self,
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        ffmpeg
            .run_checked(&[
                "-i",
                video_path.to_str().unwrap_or_default(),
                "-i",
                audio_path.to_str().unwrap_or_default(),
                "-c",
                "copy",
                "-y",
                output_path.to_str().unwrap_or_default(),
            ])
            .await
            .with_context(|| {
                format!(
                    "failed to merge {} and {} into {}",
                    video_path.display(),
                    audio_path.display(),
                    output_path.display()
                )
            })?;

        Ok(())
    }

    /// Merge fragmented DASH or HLS download into a single file
    ///
    /// Takes a list of segment files and concatenates them in order
    /// to produce a single continuous media file.
    ///
    /// # Arguments
    /// * `segments` - List of paths to segment files in order
    /// * `output_path` - Path for the merged output file
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn merge_fragmented(
        &self,
        segments: &[PathBuf],
        output_path: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        if segments.is_empty() {
            anyhow::bail!("no segments provided for merging");
        }

        // Create a concat file for ffmpeg
        let concat_content: String = segments
            .iter()
            .map(|p| format!("file '{}'\n", p.display()))
            .collect();

        let concat_file = tempfile::NamedTempFile::with_suffix(".txt")?;
        std::fs::write(concat_file.path(), concat_content)?;

        ffmpeg
            .run_checked(&[
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                concat_file.path().to_str().unwrap_or_default(),
                "-c",
                "copy",
                "-y",
                output_path.to_str().unwrap_or_default(),
            ])
            .await
            .with_context(|| {
                format!(
                    "failed to merge {} segments into {}",
                    segments.len(),
                    output_path.display()
                )
            })?;

        Ok(())
    }
}
