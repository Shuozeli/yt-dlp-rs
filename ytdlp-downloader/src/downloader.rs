use async_trait::async_trait;
use std::path::Path;

pub struct DownloadOptions {
    pub timeout: std::time::Duration,
    pub retries: u32,
    pub part_size: Option<u64>,
    pub output_template: String,
    pub proxy: String,
    pub user_agent: String,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(300),
            retries: 3,
            part_size: None,
            output_template: String::from("%(title)s-%(id)s.%(ext)s"),
            proxy: String::new(),
            user_agent: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed: f64,
    pub eta_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub output_path: std::path::PathBuf,
    pub total_bytes: u64,
}

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download(
        &self,
        url: &str,
        format_id: &str,
        dest: &Path,
        options: DownloadOptions,
        progress: impl Fn(Progress) + Send,
    ) -> anyhow::Result<DownloadResult>;
}
