//! Post-processing infrastructure for yt-dlp-rs
//!
//! This crate wraps ffmpeg to provide video/audio manipulation
//! capabilities such as merging, thumbnail embedding, and subtitle
//! conversion.

pub mod embedder;
pub mod ffmpeg;
pub mod merger;
pub mod subtitles;

pub use embedder::{Embedder, Metadata};
pub use ffmpeg::{Ffmpeg, MediaInfo, StreamInfo};
pub use merger::Merger;
pub use subtitles::SubtitlesConverter;
