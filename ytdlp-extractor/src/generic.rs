//! Generic extractor fallback

use crate::extractor::Extractor;
use crate::video_info::{Format, VideoInfo};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use url::Url;

/// Generic extractor that handles any URL as a fallback
///
/// This extractor is used when no site-specific extractor is available.
/// It extracts basic information from the URL itself without making
/// network requests.
pub struct GenericExtractor;

impl GenericExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GenericExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extractor for GenericExtractor {
    fn name(&self) -> &str {
        "generic"
    }

    fn supported_domains(&self) -> &[&str] {
        &["*"]
    }

    async fn extract(&self, url: &str) -> Result<VideoInfo> {
        let parsed_url = Url::parse(url)?;

        // Extract path components to generate a plausible ID
        let path = parsed_url.path();
        let id = if path.is_empty() || path == "/" {
            // Use host + path hash for root URLs
            format!("{}:{}", parsed_url.host_str().unwrap_or("unknown"), "root")
        } else {
            // Take last path segment as ID
            path.split('/')
                .rfind(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string()
        };

        // Create minimal video info from URL
        let formats = vec![Format {
            format_id: "generic".to_string(),
            ext: "mp4".to_string(),
            resolution: None,
            filesize: None,
            vcodec: None,
            acodec: None,
            fps: None,
            tbr: None,
            protocol: parsed_url.scheme().to_string(),
            url: Some(url.to_string()),
        }];

        Ok(VideoInfo {
            id,
            title: parsed_url
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or("Unknown")
                .to_string(),
            description: None,
            uploader: None,
            uploader_url: None,
            duration: None,
            thumbnail: None,
            formats,
            subtitles: HashMap::new(),
            thumbnails: Vec::new(),
            metadata: HashMap::new(),
        })
    }
}
