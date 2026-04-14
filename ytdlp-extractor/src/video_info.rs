//! Video information domain models

use std::collections::HashMap;
use std::time::Duration;

/// Video information extracted from a URL
#[derive(Debug, Clone, PartialEq)]
pub struct VideoInfo {
    /// Unique video identifier
    pub id: String,
    /// Video title
    pub title: String,
    /// Video description
    pub description: Option<String>,
    /// Video uploader/creator name
    pub uploader: Option<String>,
    /// URL to the uploader's channel/page
    pub uploader_url: Option<String>,
    /// Video duration
    pub duration: Option<Duration>,
    /// Primary thumbnail URL
    pub thumbnail: Option<url::Url>,
    /// Available formats for download
    pub formats: Vec<Format>,
    /// Subtitles organized by language code
    pub subtitles: HashMap<String, Vec<Subtitle>>,
    /// Additional thumbnail variants
    pub thumbnails: Vec<Thumbnail>,
    /// Additional metadata key-value pairs
    pub metadata: HashMap<String, String>,
}

/// A specific format/quality variant of a video
#[derive(Debug, Clone, PartialEq)]
pub struct Format {
    /// Unique format identifier
    pub format_id: String,
    /// File extension (e.g., "mp4", "webm")
    pub ext: String,
    /// Resolution string (e.g., "1920x1080", "720p")
    pub resolution: Option<String>,
    /// Estimated file size in bytes
    pub filesize: Option<u64>,
    /// Video codec (e.g., "h264", "vp9")
    pub vcodec: Option<String>,
    /// Audio codec (e.g., "aac", "opus")
    pub acodec: Option<String>,
    /// Frames per second
    pub fps: Option<u32>,
    /// Total bitrate in kbps
    pub tbr: Option<f64>,
    /// Download protocol (e.g., "https", "m3u8")
    pub protocol: String,
    /// Direct download URL
    pub url: Option<String>,
}

/// Subtitle track for a video
#[derive(Debug, Clone, PartialEq)]
pub struct Subtitle {
    /// Language code (e.g., "en", "es")
    pub lang: String,
    /// File extension (e.g., "vtt", "srt")
    pub ext: String,
    /// Individual subtitle entries
    pub entries: Vec<SubtitleEntry>,
}

/// A single subtitle entry with timing
#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleEntry {
    /// Start time (format depends on subtitle type)
    pub start: String,
    /// End time
    pub end: String,
    /// Subtitle text content
    pub text: String,
}

/// A thumbnail image variant
#[derive(Debug, Clone, PartialEq)]
pub struct Thumbnail {
    /// Thumbnail image URL
    pub url: url::Url,
    /// Image width in pixels
    pub width: Option<u64>,
    /// Image height in pixels
    pub height: Option<u64>,
}
