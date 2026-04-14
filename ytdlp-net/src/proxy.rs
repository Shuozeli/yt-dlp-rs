//! Proxy support for HTTP clients

use anyhow::{Context, Result};
use std::env;

/// Proxy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyType {
    Http,
    Https,
    Socks5,
    Socks5H, // SOCKS5 with hostname resolution
}

impl ProxyType {
    fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme.to_lowercase().as_str() {
            "http" => Some(ProxyType::Http),
            "https" => Some(ProxyType::Https),
            "socks5" => Some(ProxyType::Socks5),
            "socks5h" => Some(ProxyType::Socks5H),
            "socks" => Some(ProxyType::Socks5),
            _ => None,
        }
    }
}

/// A proxy configuration
#[derive(Debug, Clone)]
pub struct Proxy {
    pub url: url::Url,
    pub r#type: ProxyType,
}

impl Proxy {
    /// Parse a proxy from a URL string
    pub fn from_url(url_str: &str) -> Result<Self> {
        let url = url::Url::parse(url_str).context("Invalid proxy URL")?;
        let scheme = url.scheme();
        let proxy_type = ProxyType::from_scheme(scheme)
            .with_context(|| format!("Unknown proxy scheme: {}", scheme))?;

        Ok(Self {
            url,
            r#type: proxy_type,
        })
    }

    /// Load proxy from environment variables
    ///
    /// Checks HTTP_PROXY, HTTPS_PROXY, and NO_PROXY in order.
    pub fn from_env() -> Option<Self> {
        // Check HTTPS_PROXY first, then HTTP_PROXY
        let proxy_var = if let Ok(https) = env::var("HTTPS_PROXY") {
            if !https.is_empty() {
                Some(https)
            } else {
                None
            }
        } else if let Ok(http) = env::var("HTTP_PROXY") {
            if !http.is_empty() {
                Some(http)
            } else {
                None
            }
        } else {
            None
        };

        let proxy_str = proxy_var?;

        // Check NO_PROXY to see if this host should bypass proxy
        let no_proxy = env::var("NO_PROXY")
            .or_else(|_| env::var("no_proxy"))
            .unwrap_or_default();

        // Parse the proxy URL to check its host
        let url = url::Url::parse(&proxy_str).ok()?;
        let proxy_host = url.host_str()?;

        // Simple no_proxy check - comma-separated list of hosts/domains
        if !no_proxy.is_empty() {
            for no_proxy_item in no_proxy.split(',') {
                let item = no_proxy_item.trim();
                if item == "*" {
                    return None;
                }
                if item == proxy_host {
                    return None;
                }
                // Check if no_proxy item is a domain suffix
                if proxy_host.ends_with(item) {
                    return None;
                }
            }
        }

        let scheme = url.scheme();
        let proxy_type = ProxyType::from_scheme(scheme)?;

        Some(Proxy {
            url,
            r#type: proxy_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_from_url() {
        let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
        assert_eq!(proxy.r#type, ProxyType::Http);
        assert_eq!(proxy.url.host_str(), Some("proxy.example.com"));
        assert_eq!(proxy.url.port(), Some(8080));
    }

    #[test]
    fn test_proxy_socks5() {
        let proxy = Proxy::from_url("socks5://localhost:1080").unwrap();
        assert_eq!(proxy.r#type, ProxyType::Socks5);
    }

    #[test]
    fn test_proxy_socks5h() {
        let proxy = Proxy::from_url("socks5h://localhost:1080").unwrap();
        assert_eq!(proxy.r#type, ProxyType::Socks5H);
    }
}
