//! gRPC client for yt-dlp server

use anyhow::{Context, Result};
use std::io::Write;
use tonic::transport::Channel;
use ytdlp_proto::proto::yt_dlp_client::YtDlpClient;

use ytdlp_proto::proto::{
    DownloadRequest, DownloadSubtitlesRequest, ExtractRequest, HealthRequest, ListRequest,
    ListSubtitlesRequest,
};

const DEFAULT_SERVER_ADDR: &str = "http://127.0.0.1:50053";

/// Download options
#[derive(Default)]
pub struct DownloadOpts {
    pub retries: u32,
    pub rate_limit: Option<String>,
    pub proxy: Option<String>,
    pub user_agent: Option<String>,
    pub continue_download: bool,
}

/// Client for connecting to yt-dlp gRPC server
pub struct Client {
    client: YtDlpClient<Channel>,
}

impl Client {
    /// Connect to the yt-dlp gRPC server
    pub async fn connect(addr: &str) -> Result<Self> {
        let addr = if addr.is_empty() {
            DEFAULT_SERVER_ADDR
        } else {
            addr
        }
        .to_string();
        let channel = Channel::from_shared(addr)
            .context("Invalid server address")?
            .connect()
            .await
            .context("Failed to connect to server")?;

        let client = YtDlpClient::new(channel);
        Ok(Self { client })
    }

    /// Extract video information
    pub async fn extract(&mut self, url: &str) -> Result<ytdlp_proto::proto::VideoInfo> {
        let request = ExtractRequest {
            url: url.to_string(),
            info_fields: vec![],
            extractor_opts: Default::default(),
        };

        let response = self
            .client
            .extract(request)
            .await
            .context("Extraction failed")?;

        response
            .into_inner()
            .video
            .context("No video info in response")
    }

    /// Download a video
    pub async fn download(
        &mut self,
        url: &str,
        format_id: &str,
        output_path: &str,
        download_url: Option<&str>,
        opts: DownloadOpts,
    ) -> Result<()> {
        let request = DownloadRequest {
            url: url.to_string(),
            format_id: format_id.to_string(),
            output_path: output_path.to_string(),
            progress_interval: 0,
            download_url: download_url.map(String::from).unwrap_or_default(),
            retries: opts.retries as i32,
            rate_limit: opts.rate_limit.unwrap_or_default(),
            proxy: opts.proxy.unwrap_or_default(),
            user_agent: opts.user_agent.unwrap_or_default(),
            continue_download: opts.continue_download,
        };

        let mut stream = self
            .client
            .download(request)
            .await
            .context("Download request failed")?
            .into_inner();

        while let Some(response) = stream.message().await? {
            match response.event {
                Some(ytdlp_proto::proto::download_response::Event::Progress(progress)) => {
                    let speed = if progress.speed > 0.0 {
                        format!("{:.1} MB/s", progress.speed / 1_000_000.0)
                    } else {
                        "unknown".to_string()
                    };
                    let percent = if progress.total_bytes > 0 {
                        format!(
                            "{:.1}%",
                            (progress.downloaded_bytes as f64 / progress.total_bytes as f64)
                                * 100.0
                        )
                    } else {
                        format!("{} bytes", progress.downloaded_bytes)
                    };
                    print!("\rDownloading: {} ({}) Speed:    ", percent, speed);
                    std::io::stdout().flush().ok();
                }
                Some(ytdlp_proto::proto::download_response::Event::Completed(completed)) => {
                    println!(
                        "\nDownload complete: {} ({} bytes)",
                        completed.output_path, completed.total_bytes
                    );
                    return Ok(());
                }
                Some(ytdlp_proto::proto::download_response::Event::Error(err)) => {
                    anyhow::bail!("Download error [{}]: {}", err.code, err.error_message);
                }
                None => {}
            }
        }

        Ok(())
    }

    /// List supported sites
    pub async fn list_supported_sites(&mut self) -> Result<Vec<String>> {
        let request = ListRequest {};

        let response = self
            .client
            .list_supported_sites(request)
            .await
            .context("Failed to list sites")?;

        Ok(response.into_inner().supported_sites)
    }

    /// Health check
    pub async fn health(&mut self) -> Result<ytdlp_proto::proto::HealthResponse> {
        let request = HealthRequest {};

        let response = self
            .client
            .health(request)
            .await
            .context("Health check failed")?;

        Ok(response.into_inner())
    }

    /// List available subtitles for a video
    pub async fn list_subtitles(
        &mut self,
        url: &str,
        lang: &str,
    ) -> Result<Vec<ytdlp_proto::proto::SubtitleInfo>> {
        let request = ListSubtitlesRequest {
            url: url.to_string(),
            lang: lang.to_string(),
        };

        let response = self
            .client
            .list_subtitles(request)
            .await
            .context("Failed to list subtitles")?;

        Ok(response.into_inner().subtitles)
    }

    /// Download subtitles for a video
    pub async fn download_subtitles(
        &mut self,
        url: &str,
        lang: &str,
        output_path: &str,
        format: &str,
    ) -> Result<ytdlp_proto::proto::DownloadSubtitlesResponse> {
        let request = DownloadSubtitlesRequest {
            url: url.to_string(),
            lang: lang.to_string(),
            output_path: output_path.to_string(),
            format: format.to_string(),
        };

        let response = self
            .client
            .download_subtitles(request)
            .await
            .context("Failed to download subtitles")?;

        Ok(response.into_inner())
    }
}
