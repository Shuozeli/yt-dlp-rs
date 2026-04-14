//! Thumbnail and metadata embedder for yt-dlp-rs
//!
//! Provides functionality to embed thumbnails and metadata tags
//! into video files using ffmpeg.

use crate::ffmpeg::Ffmpeg;
use anyhow::{Context, Result};
use std::path::Path;

/// Embedder for thumbnails and metadata
pub struct Embedder;

/// Metadata tags to embed into video
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Video title
    pub title: Option<String>,
    /// Artist or creator
    pub artist: Option<String>,
    /// Album name
    pub album: Option<String>,
    /// Video description
    pub description: Option<String>,
}

impl Embedder {
    /// Embed thumbnail into video as cover art
    ///
    /// # Arguments
    /// * `video_path` - Path to the input video file
    /// * `thumbnail_path` - Path to the thumbnail image
    /// * `output_path` - Path for the output file with embedded thumbnail
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn embed_thumbnail(
        &self,
        video_path: &Path,
        thumbnail_path: &Path,
        output_path: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        ffmpeg
            .run_checked(&[
                "-i",
                video_path.to_str().unwrap_or_default(),
                "-i",
                thumbnail_path.to_str().unwrap_or_default(),
                "-c:v",
                "copy",
                "-c:a",
                "copy",
                "-attach",
                thumbnail_path.to_str().unwrap_or_default(),
                "-metadata:s:t",
                "0",
                "mimetype=image/jpeg",
                "-y",
                output_path.to_str().unwrap_or_default(),
            ])
            .await
            .with_context(|| {
                format!(
                    "failed to embed thumbnail {} into {}",
                    thumbnail_path.display(),
                    video_path.display()
                )
            })?;

        Ok(())
    }

    /// Embed metadata tags into video
    ///
    /// # Arguments
    /// * `video_path` - Path to the input video file
    /// * `metadata` - Metadata struct containing tags to embed
    /// * `output_path` - Path for the output file with embedded metadata
    /// * `ffmpeg` - Ffmpeg instance to use
    pub async fn embed_metadata(
        &self,
        video_path: &Path,
        metadata: &Metadata,
        output_path: &Path,
        ffmpeg: &Ffmpeg,
    ) -> Result<()> {
        let mut args = vec![
            "-i".to_string(),
            video_path.to_str().unwrap_or_default().to_string(),
            "-c".to_string(),
            "copy".to_string(),
            "-y".to_string(),
        ];

        if let Some(ref title) = metadata.title {
            args.push("-metadata".to_string());
            args.push(format!("title={}", title));
        }

        if let Some(ref artist) = metadata.artist {
            args.push("-metadata".to_string());
            args.push(format!("artist={}", artist));
        }

        if let Some(ref album) = metadata.album {
            args.push("-metadata".to_string());
            args.push(format!("album={}", album));
        }

        if let Some(ref description) = metadata.description {
            args.push("-metadata".to_string());
            args.push(format!("description={}", description));
        }

        args.push(output_path.to_str().unwrap_or_default().to_string());

        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        ffmpeg
            .run_checked(&args_ref)
            .await
            .with_context(|| format!("failed to embed metadata into {}", video_path.display()))?;

        Ok(())
    }
}
