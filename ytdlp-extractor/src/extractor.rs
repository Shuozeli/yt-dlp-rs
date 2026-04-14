//! Extractor trait definition

use crate::VideoInfo;
use async_trait::async_trait;

/// Trait for site-specific video extractors
///
/// Implement this trait to add support for a new video hosting site.
/// Each extractor handles one or more domains and knows how to extract
/// video information from URLs on those domains.
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Human-readable name of the extractor
    fn name(&self) -> &str;

    /// List of supported domain patterns
    ///
    /// These are used to match URLs to the appropriate extractor.
    /// Use domain without TLD prefix (e.g., "youtube.com" not "www.youtube.com").
    fn supported_domains(&self) -> &[&str];

    /// Extract video information from a URL
    ///
    /// # Arguments
    /// * `url` - The URL to extract video information from
    ///
    /// # Returns
    /// * `Ok(VideoInfo)` - Successfully extracted video information
    /// * `Err(...)` - Extraction failed
    async fn extract(&self, url: &str) -> anyhow::Result<VideoInfo>;
}
