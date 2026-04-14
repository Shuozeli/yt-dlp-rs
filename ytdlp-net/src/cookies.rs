//! Cookie jar for managing HTTP cookies

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A cookie as stored in the jar
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<std::time::SystemTime>,
    pub secure: bool,
}

/// Entry from .netrc file
#[derive(Debug)]
pub struct NetrcEntry {
    pub login: String,
    pub password: String,
    pub account: Option<String>,
}

/// Thread-safe cookie jar
#[derive(Clone)]
pub struct CookieJar {
    inner: Arc<RwLock<HashMap<String, Cookie>>>,
}

impl CookieJar {
    /// Create a new empty cookie jar
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a cookie to the jar
    pub fn add(&self, cookie: Cookie) {
        let key = cookie_key(&cookie.domain, &cookie.path, &cookie.name);
        let mut jar = self.inner.write();
        jar.insert(key, cookie);
    }

    /// Get a cookie by domain, path, and name
    pub fn get(&self, domain: &str, path: &str, name: &str) -> Option<Cookie> {
        let jar = self.inner.read();
        jar.get(&cookie_key(domain, path, name)).cloned()
    }

    /// Load cookies from a browser's cookie database
    #[cfg(unix)]
    pub fn from_browser(&self, browser: Browser) -> Result<()> {
        match browser {
            Browser::Chromium { profile } => self.load_chromium_cookies(profile),
            Browser::Firefox { profile } => self.load_firefox_cookies(profile),
        }
    }

    #[cfg(unix)]
    fn load_chromium_cookies(&self, profile: String) -> Result<()> {
        let cookie_paths = [
            dirs::data_local_dir().map(|p| p.join("google-chrome").join("Default").join("Cookies")),
            dirs::data_local_dir().map(|p| p.join("Chromium").join("Default").join("Cookies")),
            dirs::data_local_dir().map(|p| {
                p.join("google-chrome")
                    .join(profile.as_str())
                    .join("Cookies")
            }),
            dirs::data_local_dir()
                .map(|p| p.join("Chromium").join(profile.as_str()).join("Cookies")),
        ];

        let mut conn = None;
        for path in cookie_paths.into_iter().flatten() {
            if path.exists() {
                conn = Some(
                    rusqlite::Connection::open(&path).context("Failed to open Chrome cookie DB")?,
                );
                break;
            }
        }

        let conn = conn.context("Chrome cookie database not found")?;

        let mut stmt =
            conn.prepare("SELECT host, name, value, path, expires_utc, is_secure FROM cookies")?;

        let cookies = stmt.query_map([], |row| {
            let expires_utc: i64 = row.get(4)?;
            let expires = if expires_utc > 0 {
                Some(std::time::UNIX_EPOCH + std::time::Duration::from_micros(expires_utc as u64))
            } else {
                None
            };

            Ok(Cookie {
                name: row.get(1)?,
                value: row.get(2)?,
                domain: row.get(0)?,
                path: row.get(3)?,
                expires,
                secure: row.get(5)?,
            })
        })?;

        for cookie in cookies.flatten() {
            self.add(cookie);
        }

        Ok(())
    }

    #[cfg(unix)]
    fn load_firefox_cookies(&self, profile: String) -> Result<()> {
        let firefox_dir = dirs::data_dir().map(|p| p.join("mozilla").join("firefox"));

        let profile_path = if profile.is_empty() {
            // Find default profile
            let profiles_ini = firefox_dir.as_ref().map(|p| p.join("profiles.ini"));

            if let Some(profiles_ini) = profiles_ini {
                if profiles_ini.exists() {
                    // Simple parse of profiles.ini - just use first default profile
                    let content = std::fs::read_to_string(&profiles_ini)?;
                    let mut default_path = None;
                    let mut in_default = false;

                    for line in content.lines() {
                        if line.trim() == "[General]" || line.trim().starts_with("[") {
                            in_default = false;
                        }
                        if line.trim().starts_with("Default=1")
                            || line.trim().starts_with("Default=1\r")
                            || line.trim().starts_with("Default=1\n")
                        {
                            in_default = true;
                        }
                        if in_default && line.starts_with("Path=") {
                            default_path = Some(line.trim_start_matches("Path=").to_string());
                            break;
                        }
                    }

                    if let Some(path) = default_path {
                        firefox_dir.clone().map(|p| p.join(path))
                    } else {
                        firefox_dir.clone()
                    }
                } else {
                    firefox_dir.clone()
                }
            } else {
                firefox_dir.clone()
            }
        } else {
            firefox_dir.map(|p| p.join(profile))
        };

        let cookie_db = profile_path
            .map(|p| p.join("cookies.sqlite"))
            .context("No Firefox profile found")?;

        if !cookie_db.exists() {
            anyhow::bail!("Firefox cookie database not found: {:?}", cookie_db);
        }

        let conn =
            rusqlite::Connection::open(&cookie_db).context("Failed to open Firefox cookie DB")?;

        let mut stmt =
            conn.prepare("SELECT host, name, value, path, expiry, isSecure FROM moz_cookies")?;

        let cookies = stmt.query_map([], |row| {
            let expiry: i64 = row.get(4)?;

            Ok(Cookie {
                name: row.get(1)?,
                value: row.get(2)?,
                domain: row.get(0)?,
                path: row.get(3)?,
                expires: Some(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs(expiry as u64),
                ),
                secure: row.get(5)?,
            })
        })?;

        for cookie in cookies.flatten() {
            self.add(cookie);
        }

        Ok(())
    }

    /// Load authentication from .netrc file
    pub fn from_netrc(&self, machine: &str) -> Result<Option<NetrcEntry>> {
        let home = dirs::home_dir().context("Cannot find home directory")?;
        let netrc_path = home.join(".netrc");

        if !netrc_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&netrc_path)?;
        let mut entry = None;
        let mut in_machine = false;
        let mut current_login = None;
        let mut current_password = None;
        let mut current_account = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let keyword = parts[0];

            if keyword == "machine" && parts.len() >= 2 {
                if in_machine {
                    // Save previous entry if we have login
                    if let (Some(login), Some(password)) =
                        (current_login.take(), current_password.take())
                    {
                        entry = Some(NetrcEntry {
                            login,
                            password,
                            account: current_account.take(),
                        });
                    }
                    current_login = None;
                    current_password = None;
                    current_account = None;
                }
                in_machine = parts[1] == machine;
            } else if in_machine {
                match keyword {
                    "login" if parts.len() >= 2 => {
                        current_login = Some(parts[1].to_string());
                    }
                    "password" if parts.len() >= 2 => {
                        current_password = Some(parts[1].to_string());
                    }
                    "account" if parts.len() >= 2 => {
                        current_account = Some(parts[1].to_string());
                    }
                    _ => {}
                }
            }
        }

        // Don't forget the last entry
        if in_machine {
            if let (Some(login), Some(password)) = (current_login, current_password) {
                entry = Some(NetrcEntry {
                    login,
                    password,
                    account: current_account,
                });
            }
        }

        Ok(entry)
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

fn cookie_key(domain: &str, path: &str, name: &str) -> String {
    format!("{}|{}|{}", domain, path, name)
}

/// Browser types for cookie extraction
#[derive(Debug, Clone)]
pub enum Browser {
    Chromium { profile: String },
    Firefox { profile: String },
}
