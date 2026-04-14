//! HTTP client for yt-dlp

use anyhow::{Context, Result};
use bytes::Bytes;
use reqwest::{header::HeaderMap, redirect, Client, Method};
use std::time::Duration;

use super::{CookieJar, Proxy, UserAgent};

/// HTTP client options
#[derive(Debug, Clone)]
pub struct HttpOptions {
    pub user_agent: String,
    pub referer: Option<String>,
    pub timeout: Duration,
    pub follow_redirects: bool,
    pub max_redirects: u32,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            user_agent: UserAgent::random(),
            referer: None,
            timeout: Duration::from_secs(30),
            follow_redirects: true,
            max_redirects: 20,
        }
    }
}

/// HTTP response
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Bytes,
}

/// HTTP client wrapping reqwest
#[derive(Clone)]
pub struct HttpClient {
    inner: Client,
    cookies: CookieJar,
}

impl HttpClient {
    /// Create a new HTTP client with the given options
    pub fn new(options: HttpOptions) -> Result<Self> {
        let redirect_policy = if options.follow_redirects {
            redirect::Policy::limited(options.max_redirects as usize)
        } else {
            redirect::Policy::none()
        };

        let mut builder = Client::builder()
            .user_agent(&options.user_agent)
            .timeout(options.timeout)
            .redirect(redirect_policy);

        if let Some(referer) = &options.referer {
            let mut headers = HeaderMap::new();
            headers.insert(
                reqwest::header::REFERER,
                referer.parse().context("Invalid referer header")?,
            );
            builder = builder.default_headers(headers);
        }

        let inner = builder.build().context("Failed to build HTTP client")?;

        Ok(Self {
            inner,
            cookies: CookieJar::new(),
        })
    }

    /// Create a new HTTP client with proxy
    pub fn with_proxy(options: HttpOptions, proxy: &Proxy) -> Result<Self> {
        let redirect_policy = if options.follow_redirects {
            redirect::Policy::limited(options.max_redirects as usize)
        } else {
            redirect::Policy::none()
        };

        let mut builder = Client::builder()
            .user_agent(&options.user_agent)
            .timeout(options.timeout)
            .redirect(redirect_policy)
            .proxy(reqwest::Proxy::all(proxy.url.as_str())?);

        if let Some(referer) = &options.referer {
            let mut headers = HeaderMap::new();
            headers.insert(
                reqwest::header::REFERER,
                referer.parse().context("Invalid referer header")?,
            );
            builder = builder.default_headers(headers);
        }

        let inner = builder.build().context("Failed to build HTTP client")?;

        Ok(Self {
            inner,
            cookies: CookieJar::new(),
        })
    }

    /// Perform a GET request
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request(Method::GET, url, None).await
    }

    /// Perform a POST request with body
    pub async fn post(&self, url: &str, body: Bytes) -> Result<Response> {
        self.request(Method::POST, url, Some(body)).await
    }

    /// Perform a POST request with JSON body, return JSON response
    pub async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<bytes::Bytes> {
        let resp = self.post(url, Bytes::from(body)).await?;
        Ok(resp.body)
    }

    /// Perform a HEAD request
    pub async fn head(&self, url: &str) -> Result<Response> {
        self.request(Method::HEAD, url, None).await
    }

    async fn request(&self, method: Method, url: &str, body: Option<Bytes>) -> Result<Response> {
        let mut builder = self.inner.request(method, url);

        if let Some(body) = body {
            builder = builder.body(body);
        }

        let resp = builder.send().await.context("HTTP request failed")?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let body = resp.bytes().await.context("Failed to read response body")?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }

    /// Return a new client with additional cookies
    pub fn with_cookies(&self, cookies: CookieJar) -> Self {
        Self {
            inner: self.inner.clone(),
            cookies,
        }
    }

    /// Get the underlying cookie jar
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }
}
