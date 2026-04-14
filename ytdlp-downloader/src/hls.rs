use anyhow::{bail, Context};
use async_trait::async_trait;
use hls_m3u8::tags::VariantStream;
use hls_m3u8::{MasterPlaylist, MediaPlaylist};
use reqwest::Client;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

use super::downloader::{DownloadOptions, DownloadResult, Downloader, Progress};

pub struct HlsDownloader {
    client: Client,
}

impl HlsDownloader {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to create HTTP client");
        Self { client }
    }
}

impl Default for HlsDownloader {
    fn default() -> Self {
        Self::new()
    }
}

fn get_variant_uri(variant: &VariantStream) -> String {
    match variant {
        VariantStream::ExtXStreamInf { uri, .. } => uri.to_string(),
        VariantStream::ExtXIFrame { uri, .. } => uri.to_string(),
    }
}

fn parse_m3u8(content: &str) -> anyhow::Result<Vec<String>> {
    // Check if this is a master playlist
    if content.contains("#EXT-X-STREAM-INF") {
        let master =
            MasterPlaylist::try_from(content).context("failed to parse master playlist")?;
        // For now, pick the first variant stream
        if let Some(variant) = master.variant_streams.first() {
            Ok(vec![get_variant_uri(variant)])
        } else {
            bail!("No variant streams found in master playlist");
        }
    } else {
        // Media playlist
        let media = MediaPlaylist::try_from(content).context("failed to parse media playlist")?;

        let segments: Vec<String> = media
            .segments
            .iter()
            .map(|(_idx, seg)| seg.uri().to_string())
            .collect();

        Ok(segments)
    }
}

async fn download_segment(client: &Client, url: &str) -> anyhow::Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .context("failed to download segment")?;

    if !response.status().is_success() {
        bail!("Failed to download segment: {}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .context("failed to read segment bytes")?;
    Ok(bytes.to_vec())
}

#[async_trait]
impl Downloader for HlsDownloader {
    async fn download(
        &self,
        url: &str,
        _format_id: &str,
        dest: &Path,
        _options: DownloadOptions,
        progress: impl Fn(Progress) + Send,
    ) -> anyhow::Result<DownloadResult> {
        // Download the m3u8 playlist
        let playlist_response = self
            .client
            .get(url)
            .send()
            .await
            .context("failed to fetch m3u8 playlist")?;
        let playlist_content = playlist_response
            .text()
            .await
            .context("failed to read playlist content")?;

        // For master playlists, we need to fetch a variant playlist
        let segments = if playlist_content.contains("#EXT-X-STREAM-INF") {
            // Master playlist - extract variant URL
            let master = MasterPlaylist::try_from(playlist_content.as_str())
                .context("failed to parse master playlist")?;
            if let Some(variant) = master.variant_streams.first() {
                let variant_uri = get_variant_uri(variant);
                // Fetch the variant playlist
                let variant_response = self
                    .client
                    .get(&variant_uri)
                    .send()
                    .await
                    .context("failed to fetch variant playlist")?;
                let variant_content = variant_response
                    .text()
                    .await
                    .context("failed to read variant content")?;
                parse_m3u8(&variant_content)?
            } else {
                bail!("No variant streams found");
            }
        } else {
            parse_m3u8(&playlist_content)?
        };

        let total_segments = segments.len();
        let start = Instant::now();
        let mut total_bytes = 0u64;

        // Create output file
        let mut file = tokio::fs::File::create(dest)
            .await
            .context("failed to create output file")?;

        for (i, segment_url) in segments.iter().enumerate() {
            tracing::debug!(
                "Downloading segment {}/{}: {}",
                i + 1,
                total_segments,
                segment_url
            );

            let segment_data = download_segment(&self.client, segment_url)
                .await
                .with_context(|| format!("failed to download segment {}", i + 1))?;

            file.write_all(&segment_data)
                .await
                .context("failed to write segment to file")?;

            total_bytes += segment_data.len() as u64;

            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                total_bytes as f64 / elapsed
            } else {
                0.0
            };
            let remaining_segments = total_segments - i - 1;
            let eta = if speed > 0.0 && remaining_segments > 0 {
                Some((remaining_segments as f64 * (total_bytes as f64 / (i + 1) as f64)) / speed)
            } else {
                None
            };

            progress(Progress {
                downloaded_bytes: total_bytes,
                total_bytes: None, // HLS total unknown upfront
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
