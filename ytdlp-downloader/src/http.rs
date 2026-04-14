use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Proxy};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

use super::downloader::{DownloadOptions, DownloadResult, Downloader, Progress};

pub struct HttpDownloader {
    client: Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        Self::with_options(&DownloadOptions::default()).expect("failed to create HTTP client")
    }

    pub fn with_options(options: &DownloadOptions) -> anyhow::Result<Self> {
        let mut builder = Client::builder().timeout(Duration::from_secs(300));

        // Apply proxy if specified
        if !options.proxy.is_empty() {
            builder = builder.proxy(Proxy::all(&options.proxy)?);
        }

        // Apply user agent if specified
        if !options.user_agent.is_empty() {
            builder = builder.user_agent(&options.user_agent);
        }

        let client = builder.build()?;
        Ok(Self { client })
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Downloader for HttpDownloader {
    async fn download(
        &self,
        url: &str,
        _format_id: &str,
        dest: &Path,
        _options: DownloadOptions,
        progress: impl Fn(Progress) + Send,
    ) -> anyhow::Result<DownloadResult> {
        let start = Instant::now();
        let mut total_bytes = 0u64;

        // Handle resume if file exists
        let existing_size = if dest.exists() {
            tokio::fs::metadata(dest).await.map(|m| m.len()).ok()
        } else {
            None
        };

        let mut request = self.client.get(url);

        if let Some(size) = existing_size {
            request = request.header("Range", format!("bytes={}-", size));
        }

        let mut response = request.send().await.context("failed to send request")?;

        let status = response.status();
        if !status.is_success() && status.as_u16() != 206 {
            anyhow::bail!("HTTP request failed with status: {}", status);
        }

        let content_length = response.content_length();

        let total_bytes_to_download = if let Some(size) = existing_size {
            content_length.map(|c| c + size).unwrap_or(0)
        } else {
            content_length.unwrap_or(0)
        };

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(existing_size.is_none())
            .open(dest)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to open destination file '{}': {:?}",
                    dest.display(),
                    e
                )
            })?;

        let mut downloaded = existing_size.unwrap_or(0);

        while let Some(chunk) = response.chunk().await.context("failed to read chunk")? {
            let chunk_len = chunk.len();
            file.write_all(&chunk)
                .await
                .context("failed to write chunk")?;
            downloaded += chunk_len as u64;
            total_bytes += chunk_len as u64;

            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };
            let eta = if speed > 0.0 {
                Some((total_bytes_to_download.saturating_sub(downloaded)) as f64 / speed)
            } else {
                None
            };

            progress(Progress {
                downloaded_bytes: downloaded,
                total_bytes: if total_bytes_to_download > 0 {
                    Some(total_bytes_to_download)
                } else {
                    None
                },
                speed,
                eta_seconds: eta,
            });
        }

        file.flush().await.context("failed to flush file")?;

        Ok(DownloadResult {
            output_path: dest.to_path_buf(),
            total_bytes,
        })
    }
}
