//! Extractor registry for mapping URLs to extractors

use crate::extractor::Extractor;
use std::collections::HashMap;
use std::sync::RwLock;
use url::Url;

/// Global registry of extractors keyed by domain
static REGISTRY: RwLock<Option<HashMap<String, &'static dyn Extractor>>> = RwLock::new(None);

/// Thread-safe registry for mapping domains to extractors
pub struct ExtractorRegistry;

impl ExtractorRegistry {
    /// Register an extractor with the global registry
    ///
    /// This is typically called during initialization of site-specific
    /// extractor modules.
    pub fn register(extractor: &'static dyn Extractor) {
        let mut registry = REGISTRY.write().unwrap();
        if registry.is_none() {
            *registry = Some(HashMap::new());
        }
        let registry = registry.as_mut().unwrap();
        for domain in extractor.supported_domains() {
            registry.insert(domain.to_lowercase(), extractor);
        }
    }

    /// Find an extractor that can handle the given URL
    ///
    /// Returns the extractor if found, None otherwise.
    pub fn for_url(url: &str) -> Option<&'static dyn Extractor> {
        let registry = REGISTRY.read().unwrap();
        let registry = registry.as_ref()?;

        let url = Url::parse(url).ok()?;
        let host = url.host_str()?.to_lowercase();

        // Try exact match first
        if let Some(extractor) = registry.get(&host) {
            return Some(*extractor);
        }

        // Try without www prefix
        let host_without_www = host.strip_prefix("www.").unwrap_or(&host);
        registry.get(host_without_www).copied()
    }
}
