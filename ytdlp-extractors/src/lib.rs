//! Site extractors for yt-dlp-rs
//!
//! This crate provides site-specific extractors for various video platforms.

pub mod generic;
pub mod youtube;

pub use generic::GenericExtractor;
pub use youtube::YoutubeExtractor;

use std::sync::Arc;
use ytdlp_extractor::Extractor;

/// Get all built-in extractors
pub fn all_extractors() -> Vec<Arc<dyn Extractor>> {
    vec![
        Arc::new(YoutubeExtractor::new()) as Arc<dyn Extractor>,
        Arc::new(GenericExtractor::new()) as Arc<dyn Extractor>,
    ]
}
