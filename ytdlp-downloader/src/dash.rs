use anyhow::{bail, Context};
use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

use super::downloader::{DownloadOptions, DownloadResult, Downloader, Progress};

pub struct DashDownloader {
    client: Client,
}

impl DashDownloader {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to create HTTP client");
        Self { client }
    }
}

impl Default for DashDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DashSegment {
    url: String,
    initialization: Option<String>,
}

#[allow(unused_variables, unused_assignments)]
fn parse_mpd(manifest: &str) -> anyhow::Result<DashManifest> {
    let mut reader = Reader::from_str(manifest);
    reader.config_mut().trim_text(true);

    let mut dash_manifest = DashManifest::default();
    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut in_adaptation_set = false;
    let mut in_representation = false;
    let mut in_base_url = false;
    let mut base_url = String::new();
    let mut current_rep_id = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_element = name.clone();

                match name.as_str() {
                    "AdaptationSet" => {
                        in_adaptation_set = true;
                    }
                    "Representation" => {
                        in_representation = true;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            if key == "id" {
                                current_rep_id = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "BaseURL" => {
                        in_base_url = true;
                    }
                    "SegmentTemplate" => {
                        // Parse SegmentTemplate for initialization and media URLs
                        let mut init_url = None;
                        let mut media_url = None;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "initialization" => init_url = Some(value),
                                "media" => media_url = Some(value),
                                _ => {}
                            }
                        }
                        if let (Some(init), Some(media)) = (init_url, media_url) {
                            dash_manifest.segment_template = Some(SegmentTemplate {
                                initialization: init,
                                media,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if name == "Representation" && in_adaptation_set {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        if key == "id" {
                            current_rep_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_base_url {
                    base_url = e.unescape().unwrap_or_default().to_string();
                    in_base_url = false;
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "AdaptationSet" => {
                        in_adaptation_set = false;
                    }
                    "Representation" => {
                        in_representation = false;
                        if !base_url.is_empty() && !current_rep_id.is_empty() {
                            dash_manifest.representations.push(Representation {
                                id: current_rep_id.clone(),
                                base_url: base_url.clone(),
                            });
                        }
                        current_rep_id.clear();
                    }
                    "BaseURL" => {
                        in_base_url = false;
                    }
                    _ => {}
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("Error parsing MPD: {:?}", e),
            _ => {}
        }
        buf.clear();
    }

    Ok(dash_manifest)
}

#[derive(Debug, Default)]
struct DashManifest {
    representations: Vec<Representation>,
    segment_template: Option<SegmentTemplate>,
}

#[derive(Debug, Clone)]
struct Representation {
    id: String,
    base_url: String,
}

#[derive(Debug, Clone)]
struct SegmentTemplate {
    initialization: String,
    media: String,
}

async fn download_data(client: &Client, url: &str) -> anyhow::Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .context("failed to download data")?;

    if !response.status().is_success() {
        bail!("Failed to download: {}", response.status());
    }

    let bytes = response.bytes().await.context("failed to read bytes")?;
    Ok(bytes.to_vec())
}

#[async_trait]
impl Downloader for DashDownloader {
    async fn download(
        &self,
        url: &str,
        format_id: &str,
        dest: &Path,
        _options: DownloadOptions,
        progress: impl Fn(Progress) + Send,
    ) -> anyhow::Result<DownloadResult> {
        // Download the MPD manifest
        let manifest_content = download_data(&self.client, url)
            .await
            .context("failed to download MPD manifest")?;
        let manifest_str =
            String::from_utf8(manifest_content).context("failed to parse manifest as UTF-8")?;

        let manifest = parse_mpd(&manifest_str).context("failed to parse DASH manifest")?;

        // Select representation based on format_id or use first one
        let rep = if !format_id.is_empty() {
            manifest
                .representations
                .iter()
                .find(|r| r.id == format_id)
                .cloned()
                .unwrap_or_else(|| manifest.representations.first().cloned().unwrap())
        } else {
            manifest
                .representations
                .first()
                .cloned()
                .context("No representations found in manifest")?
        };

        // Download initialization segment if present
        if let Some(ref template) = manifest.segment_template {
            let init_url = template
                .initialization
                .replace("$RepresentationID$", &rep.id);

            tracing::debug!("Downloading init segment: {}", init_url);
            let init_data = download_data(&self.client, &init_url)
                .await
                .context("failed to download init segment")?;

            // TODO: Write init segment - for now we just track bytes
            let _ = init_data.len();
        }

        // For now, download the first segment as a demo
        // Full implementation would iterate through all segments
        if let Some(ref template) = manifest.segment_template {
            let segment_url = template
                .media
                .replace("$RepresentationID$", &rep.id)
                .replace("$Number$", "1");

            let full_url = if segment_url.starts_with("http") {
                segment_url
            } else {
                // Resolve relative URL
                if let Some(slash_pos) = rep.base_url.rfind('/') {
                    format!("{}/{}", &rep.base_url[..slash_pos + 1], segment_url)
                } else {
                    format!("{}/{}", rep.base_url, segment_url)
                }
            };

            let start = Instant::now();
            let segment_data = download_data(&self.client, &full_url)
                .await
                .context("failed to download segment")?;

            let total_bytes = segment_data.len() as u64;

            let mut file = tokio::fs::File::create(dest)
                .await
                .context("failed to create output file")?;
            file.write_all(&segment_data)
                .await
                .context("failed to write segment to file")?;
            file.flush().await.context("failed to flush file")?;

            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                total_bytes as f64 / elapsed
            } else {
                0.0
            };

            progress(Progress {
                downloaded_bytes: total_bytes,
                total_bytes: None,
                speed,
                eta_seconds: None,
            });

            Ok(DownloadResult {
                output_path: dest.to_path_buf(),
                total_bytes,
            })
        } else {
            bail!("No segment template found in manifest");
        }
    }
}
