//! yt-dlp-extractor
//!
//! Core extraction framework for video downloader.
//!
//! This crate provides the framework for extracting video information from various
//! video hosting sites. It defines the core traits and types used by
//! site-specific extractors.

pub mod extractor;
pub mod generic;
pub mod registry;
pub mod video_info;

pub use extractor::Extractor;
pub use registry::ExtractorRegistry;
pub use video_info::{Format, Subtitle, SubtitleEntry, Thumbnail, VideoInfo};
