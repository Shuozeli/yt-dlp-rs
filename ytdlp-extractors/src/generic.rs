//! Generic extractor for yt-dlp-rs
//!
//! A fallback extractor that handles URLs that don't match any site-specific extractor.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use ytdlp_extractor::{Extractor, Format, VideoInfo};
use ytdlp_net::{HttpClient, HttpOptions};

/// Generic fallback extractor
pub struct GenericExtractor {
    http: HttpClient,
}

impl GenericExtractor {
    pub fn new() -> Self {
        let options = HttpOptions::default();
        let http = HttpClient::new(options).expect("failed to create HTTP client");
        Self { http }
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
        &[]
    }

    async fn extract(&self, url: &str) -> Result<VideoInfo> {
        let parsed = url::Url::parse(url)?;
        let path = parsed.path();
        let filename = path.split('/').next_back().unwrap_or("video").to_string();

        // Determine extension from URL
        let ext = if filename.contains('.') {
            filename
                .split('.')
                .next_back()
                .unwrap_or("mp4")
                .to_lowercase()
        } else {
            "mp4".to_string()
        };

        // Try to get content-length from HEAD request for filesize
        let filesize = self.fetch_content_length(url).await.ok();

        let format = Format {
            format_id: "generic".to_string(),
            ext: ext.clone(),
            resolution: None,
            filesize,
            vcodec: None,
            acodec: None,
            fps: None,
            tbr: None,
            protocol: parsed.scheme().to_string(),
            url: Some(url.to_string()),
        };

        Ok(VideoInfo {
            id: parsed.path().to_string(),
            title: filename,
            description: None,
            uploader: None,
            uploader_url: None,
            duration: None,
            thumbnail: None,
            formats: vec![format],
            subtitles: HashMap::new(),
            thumbnails: vec![],
            metadata: HashMap::from([
                ("source_url".to_string(), url.to_string()),
                ("extractor".to_string(), "generic".to_string()),
            ]),
        })
    }
}

impl GenericExtractor {
    async fn fetch_content_length(&self, url: &str) -> Result<u64> {
        // Try HEAD request first
        let response = self.http.head(url).await.ok();
        if let Some(resp) = response {
            if let Some(cl) = resp.headers.get("content-length") {
                let cl_str = cl.to_str().unwrap_or("0");
                return Ok(cl_str.parse().unwrap_or(0));
            }
        }

        // Fall back to GET response headers
        let response = self.http.get(url).await.ok();
        if let Some(resp) = response {
            if let Some(cl) = resp.headers.get("content-length") {
                let cl_str = cl.to_str().unwrap_or("0");
                return Ok(cl_str.parse().unwrap_or(0));
            }
        }

        anyhow::bail!("Could not determine content length")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_extractor_name() {
        let extractor = GenericExtractor::new();
        assert_eq!(extractor.name(), "generic");
    }
}
