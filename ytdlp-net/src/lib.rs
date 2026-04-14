//! yt-dlp networking layer
//!
//! HTTP client, cookie jar, proxy support, and user-agent management.

pub mod cookies;
pub mod http;
pub mod proxy;
pub mod redirect;
pub mod user_agent;

pub use cookies::{Cookie, CookieJar, NetrcEntry};
pub use http::{HttpClient, HttpOptions, Response};
pub use proxy::{Proxy, ProxyType};
pub use user_agent::UserAgent;
