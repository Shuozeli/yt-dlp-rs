//! User-Agent string management

use rand::seq::SliceRandom;
use rand::thread_rng;

/// Desktop browser User-Agent strings
const CHROME_UA: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
];

const FIREFOX_UA: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:119.0) Gecko/20100101 Firefox/119.0",
];

const SAFARI_UA: &[&str] = &[
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15",
];

const EDGE_UA: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36 Edg/118.0.0.0",
];

/// User-Agent management
pub struct UserAgent;

impl UserAgent {
    /// Get a random desktop browser User-Agent string
    pub fn random() -> String {
        let all_uas: Vec<&[&str]> = vec![CHROME_UA, FIREFOX_UA, SAFARI_UA, EDGE_UA];
        let chosen = all_uas
            .choose(&mut thread_rng())
            .expect("UA list is non-empty");
        let ua = chosen
            .choose(&mut thread_rng())
            .expect("Browser UA list is non-empty");
        ua.to_string()
    }

    /// Get a User-Agent string suitable for a specific extractor
    ///
    /// Some extractors require specific UA strings to work correctly.
    pub fn for_extractor(_extractor: &str) -> String {
        // Most extractors work fine with the default random UA
        // but some may need specific handling
        Self::random()
    }

    /// Get a Chrome User-Agent
    pub fn chrome() -> String {
        CHROME_UA
            .choose(&mut thread_rng())
            .expect("Chrome UA list is non-empty")
            .to_string()
    }

    /// Get a Firefox User-Agent
    pub fn firefox() -> String {
        FIREFOX_UA
            .choose(&mut thread_rng())
            .expect("Firefox UA list is non-empty")
            .to_string()
    }

    /// Get a Safari User-Agent
    pub fn safari() -> String {
        SAFARI_UA
            .choose(&mut thread_rng())
            .expect("Safari UA list is non-empty")
            .to_string()
    }

    /// Get an Edge User-Agent
    pub fn edge() -> String {
        EDGE_UA
            .choose(&mut thread_rng())
            .expect("Edge UA list is non-empty")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_ua() {
        let ua = UserAgent::random();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla/5.0"));
    }

    #[test]
    fn test_chrome_ua() {
        let ua = UserAgent::chrome();
        assert!(ua.contains("Chrome"));
    }

    #[test]
    fn test_firefox_ua() {
        let ua = UserAgent::firefox();
        assert!(ua.contains("Firefox"));
    }
}
