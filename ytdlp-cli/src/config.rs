//! Configuration file support for ytdlp-cli.
//!
//! Loads configuration from `~/.config/ytdlp-rs.toml`.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// CLI configuration loaded from file.
#[derive(Debug, Clone)]
pub struct Config {
    /// Default output template for downloads.
    pub output_template: String,
    /// Default number of retries.
    pub retries: u32,
    /// Default rate limit (e.g., "1M" for 1 MB/s).
    pub rate_limit: Option<String>,
    /// Default proxy URL.
    pub proxy: Option<String>,
    /// Default user agent string.
    pub user_agent: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_template: "%(title)s-%(id)s.%(ext)s".to_string(),
            retries: 10,
            rate_limit: None,
            proxy: None,
            user_agent: None,
        }
    }
}

impl Config {
    /// Load configuration from the default config file path.
    ///
    /// Returns `Ok(None)` if the config file doesn't exist.
    pub fn load() -> Result<Option<Self>> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(None);
        }

        Self::load_from_file(&config_path)
    }

    /// Load configuration from a specific file path.
    pub fn load_from_file(path: &PathBuf) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {:?}", path))?;

        let config: FileConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {:?}", path))?;

        Ok(Some(config.into()))
    }

    /// Get the default configuration file path.
    ///
    /// Resolves to `~/.config/ytdlp-rs.toml`.
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;

        let config_dir = home.join(".config");
        let config_file = config_dir.join("ytdlp-rs.toml");

        Ok(config_file)
    }

    /// Get the config directory path.
    #[allow(dead_code)]
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;

        Ok(home.join(".config"))
    }
}

/// Internal config structure matching TOML file format.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct FileConfig {
    #[serde(default)]
    output_template: Option<String>,
    #[serde(default)]
    retries: Option<u32>,
    #[serde(default)]
    rate_limit: Option<String>,
    #[serde(default)]
    proxy: Option<String>,
    #[serde(default)]
    user_agent: Option<String>,
}

impl From<FileConfig> for Config {
    fn from(fc: FileConfig) -> Self {
        Self {
            output_template: fc
                .output_template
                .unwrap_or_else(|| "%(title)s-%(id)s.%(ext)s".to_string()),
            retries: fc.retries.unwrap_or(10),
            rate_limit: fc.rate_limit,
            proxy: fc.proxy,
            user_agent: fc.user_agent,
        }
    }
}

/// Save configuration to the default config file path.
#[allow(dead_code)]
pub fn save_config(config: &Config) -> Result<PathBuf> {
    let config_path = Config::config_path()?;

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let toml_content = toml::to_string_pretty(&FileConfig {
        output_template: Some(config.output_template.clone()),
        retries: Some(config.retries),
        rate_limit: config.rate_limit.clone(),
        proxy: config.proxy.clone(),
        user_agent: config.user_agent.clone(),
    })?;

    fs::write(&config_path, toml_content).context("Failed to write config file")?;

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("ytdlp-rs.toml");

        let toml_content = r#"
output_template = "%(title)s.%(ext)s"
retries = 5
rate_limit = "5M"
"#;

        let mut file = File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::load_from_file(&config_path).unwrap().unwrap();

        assert_eq!(config.output_template, "%(title)s.%(ext)s");
        assert_eq!(config.retries, 5);
        assert_eq!(config.rate_limit, Some("5M".to_string()));
    }

    #[test]
    fn test_config_defaults() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("ytdlp-rs.toml");

        let toml_content = r#"
"#;

        let mut file = File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::load_from_file(&config_path).unwrap().unwrap();

        assert_eq!(config.output_template, "%(title)s-%(id)s.%(ext)s");
        assert_eq!(config.retries, 10);
    }

    #[test]
    fn test_config_path() {
        // This test will fail if home dir can't be determined
        if let Ok(path) = Config::config_path() {
            assert!(path.ends_with("ytdlp-rs.toml"));
        }
    }
}
