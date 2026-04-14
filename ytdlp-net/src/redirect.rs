//! Redirect handling utilities

/// Redirect policy configuration
#[derive(Debug, Clone)]
pub struct RedirectPolicy {
    pub max_redirects: u32,
    pub follow_redirects: bool,
    pub strict_redirect: bool,
}

impl Default for RedirectPolicy {
    fn default() -> Self {
        Self {
            max_redirects: 20,
            follow_redirects: true,
            strict_redirect: false,
        }
    }
}

impl RedirectPolicy {
    /// Create a new redirect policy
    pub fn new(max_redirects: u32, follow_redirects: bool) -> Self {
        Self {
            max_redirects,
            follow_redirects,
            strict_redirect: false,
        }
    }

    /// Set whether to strictly follow redirects (no POST->GET conversion)
    pub fn with_strict(self, strict: bool) -> Self {
        Self {
            strict_redirect: strict,
            ..self
        }
    }
}

/// Information about a redirect
#[derive(Debug)]
pub struct RedirectInfo {
    pub status: u16,
    pub location: String,
    pub from_url: String,
    pub to_url: String,
}

impl RedirectInfo {
    /// Check if this is a redirect to a different domain
    pub fn is_cross_domain(&self) -> bool {
        let from_host = url::Url::parse(&self.from_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()));
        let to_host = url::Url::parse(&self.to_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()));

        from_host != to_host
    }

    /// Check if this is a redirect from HTTPS to HTTP (security issue)
    pub fn is_downgrade(&self) -> bool {
        let from = url::Url::parse(&self.from_url).ok();
        let to = url::Url::parse(&self.to_url).ok();

        from.zip(to)
            .map(|(f, t)| f.scheme() == "https" && t.scheme() == "http")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_domain_redirect() {
        let info = RedirectInfo {
            status: 302,
            location: "https://other.com/page".to_string(),
            from_url: "https://example.com/page".to_string(),
            to_url: "https://other.com/page".to_string(),
        };
        assert!(info.is_cross_domain());
    }

    #[test]
    fn test_same_domain_redirect() {
        let info = RedirectInfo {
            status: 301,
            location: "/different-page".to_string(),
            from_url: "https://example.com/page".to_string(),
            to_url: "https://example.com/different-page".to_string(),
        };
        assert!(!info.is_cross_domain());
    }

    #[test]
    fn test_downgrade_redirect() {
        let info = RedirectInfo {
            status: 301,
            location: "http://example.com/page".to_string(),
            from_url: "https://example.com/page".to_string(),
            to_url: "http://example.com/page".to_string(),
        };
        assert!(info.is_downgrade());
    }
}
