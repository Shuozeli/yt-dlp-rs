//! gRPC service implementation

use anyhow::Result;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use ytdlp_proto::proto::{
    yt_dlp_server::YtDlp, DownloadRequest, DownloadResponse, DownloadSubtitlesRequest,
    DownloadSubtitlesResponse, ExtractRequest, ExtractResponse, Format as ProtoFormat,
    HealthRequest, HealthResponse, ListRequest, ListResponse, ListSubtitlesRequest,
    ListSubtitlesResponse, Subtitle as ProtoSubtitle, SubtitleInfo, Thumbnail as ProtoThumbnail,
};

use ytdlp_downloader::{DownloadOptions, Downloader, HttpDownloader};
use ytdlp_extractor::Extractor;
use ytdlp_extractors::{all_extractors, GenericExtractor};

type DownloadStream = Pin<Box<dyn Stream<Item = Result<DownloadResponse, Status>> + Send>>;

pub struct YtDlpService {
    extractors: Vec<Arc<dyn Extractor>>,
}

impl YtDlpService {
    pub fn new() -> Self {
        Self {
            extractors: all_extractors(),
        }
    }

    fn find_extractor(&self, url: &str) -> Option<Arc<dyn Extractor>> {
        for extractor in &self.extractors {
            for domain in extractor.supported_domains() {
                if url.contains(domain) {
                    return Some(Arc::clone(extractor));
                }
            }
        }
        None
    }

    fn proto_video_info(info: ytdlp_extractor::VideoInfo) -> ytdlp_proto::proto::VideoInfo {
        let formats = info
            .formats
            .into_iter()
            .map(|f| ProtoFormat {
                format_id: f.format_id,
                ext: f.ext,
                resolution: f.resolution.unwrap_or_default(),
                filesize: f.filesize.unwrap_or(0) as i64,
                vcodec: f.vcodec.unwrap_or_default(),
                acodec: f.acodec.unwrap_or_default(),
                fps: f.fps.unwrap_or(0) as i64,
                tbr: f.tbr.map(|t| t.to_string()).unwrap_or_default(),
                protocol: f.protocol,
                url: f.url.unwrap_or_default(),
            })
            .collect();

        let subtitles = info
            .subtitles
            .into_iter()
            .flat_map(|(lang, subtitles)| {
                subtitles.into_iter().map(move |s| ProtoSubtitle {
                    lang: lang.clone(),
                    ext: s.ext,
                    entries: s
                        .entries
                        .into_iter()
                        .map(|e| ytdlp_proto::proto::SubtitleEntry {
                            start: e.start,
                            end: e.end,
                            text: e.text,
                        })
                        .collect(),
                })
            })
            .collect::<Vec<_>>();

        let thumbnails = info
            .thumbnails
            .into_iter()
            .map(|t| ProtoThumbnail {
                url: t.url.to_string(),
                width: t.width.unwrap_or(0) as i64,
                height: t.height.unwrap_or(0) as i64,
            })
            .collect();

        ytdlp_proto::proto::VideoInfo {
            id: info.id,
            title: info.title,
            description: info.description.unwrap_or_default(),
            uploader: info.uploader.unwrap_or_default(),
            uploader_url: info.uploader_url.unwrap_or_default(),
            duration: info.duration.map(|d| d.as_secs() as i64).unwrap_or(0),
            metadata: info.metadata,
            formats,
            subtitles,
            thumbnails,
        }
    }
}

#[tonic::async_trait]
impl YtDlp for YtDlpService {
    type DownloadStream = DownloadStream;

    async fn extract(
        &self,
        request: Request<ExtractRequest>,
    ) -> Result<Response<ExtractResponse>, Status> {
        let req = request.into_inner();
        let url = req.url;

        tracing::info!("Extracting info for URL: {}", url);

        let extractor: Arc<dyn Extractor> = match self.find_extractor(&url) {
            Some(e) => e,
            None => Arc::new(GenericExtractor::new()) as Arc<dyn Extractor>,
        };

        match extractor.extract(&url).await {
            Ok(info) => {
                let proto_info = Self::proto_video_info(info);
                Ok(Response::new(ExtractResponse {
                    video: Some(proto_info),
                }))
            }
            Err(e) => {
                tracing::error!("Extraction failed: {}", e);
                Err(Status::internal(format!("Extraction failed: {}", e)))
            }
        }
    }

    async fn download(
        &self,
        request: Request<DownloadRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        let req = request.into_inner();
        let url = req.url.clone();
        let format_id = req.format_id;
        let output_path = req.output_path.clone();

        tracing::info!("Download requested: {} (format: {})", url, format_id);
        tracing::info!("Output path: {:?}", output_path);

        // Use provided download_url if available, otherwise re-extract
        let download_url = if !req.download_url.is_empty() {
            req.download_url.clone()
        } else {
            tracing::info!("No download_url provided, re-extracting video info...");
            let extractor: Arc<dyn Extractor> = match self.find_extractor(&url) {
                Some(e) => e,
                None => Arc::new(GenericExtractor::new()) as Arc<dyn Extractor>,
            };

            let video_info = extractor
                .extract(&url)
                .await
                .map_err(|e| Status::internal(format!("Extraction failed: {}", e)))?;

            let format = video_info
                .formats
                .iter()
                .find(|f| f.format_id == format_id)
                .ok_or_else(|| {
                    Status::invalid_argument(format!("Format {} not found", format_id))
                })?;

            format
                .url
                .clone()
                .ok_or_else(|| Status::invalid_argument("Format has no download URL"))?
        };

        let dest = std::path::PathBuf::from(output_path);
        tracing::info!("Destination path: {:?}", dest);
        tracing::info!("Destination exists: {:?}", dest.exists());
        if let Some(parent) = dest.parent() {
            tracing::info!("Parent directory: {:?}", parent);
            tracing::info!("Parent exists: {:?}", parent.exists());
        }
        let (tx, rx) = mpsc::channel(128);
        let stream = ReceiverStream::new(rx);

        // Build download options
        let options = DownloadOptions {
            retries: if req.retries > 0 {
                req.retries as u32
            } else {
                3
            },
            proxy: req.proxy.clone(),
            user_agent: req.user_agent.clone(),
            ..Default::default()
        };

        // Create downloader with options (for proxy/user_agent support)
        let http_downloader = match HttpDownloader::with_options(&options) {
            Ok(d) => d,
            Err(e) => {
                return Err(Status::internal(format!(
                    "Failed to create downloader: {}",
                    e
                )));
            }
        };

        tokio::spawn(async move {
            let tx_progress = tx.clone();
            let result = http_downloader
                .download(&download_url, &format_id, &dest, options, move |progress| {
                    let _ = tx_progress.try_send(Ok(DownloadResponse {
                        event: Some(ytdlp_proto::proto::download_response::Event::Progress(
                            ytdlp_proto::proto::Progress {
                                downloaded_bytes: progress.downloaded_bytes as i64,
                                total_bytes: progress.total_bytes.unwrap_or(0) as i64,
                                speed: progress.speed as f32,
                                eta: progress.eta_seconds.unwrap_or(0.0) as f32,
                            },
                        )),
                    }));
                })
                .await;

            match result {
                Ok(download_result) => {
                    let _ = tx
                        .send(Ok(DownloadResponse {
                            event: Some(ytdlp_proto::proto::download_response::Event::Completed(
                                ytdlp_proto::proto::Completed {
                                    output_path: download_result
                                        .output_path
                                        .to_string_lossy()
                                        .to_string(),
                                    total_bytes: download_result.total_bytes as i64,
                                },
                            )),
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(DownloadResponse {
                            event: Some(ytdlp_proto::proto::download_response::Event::Error(
                                ytdlp_proto::proto::DownloadError {
                                    code: "download_failed".to_string(),
                                    error_message: e.to_string(),
                                },
                            )),
                        }))
                        .await;
                }
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn list_supported_sites(
        &self,
        _request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        let sites: Vec<String> = self
            .extractors
            .iter()
            .map(|e| e.name().to_string())
            .collect();

        Ok(Response::new(ListResponse {
            supported_sites: sites,
        }))
    }

    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn list_subtitles(
        &self,
        request: Request<ListSubtitlesRequest>,
    ) -> Result<Response<ListSubtitlesResponse>, Status> {
        let req = request.into_inner();
        let url = req.url;

        tracing::info!("Listing subtitles for URL: {}", url);

        // Run yt-dlp --list-subs to get available subtitles
        let output = tokio::process::Command::new("yt-dlp")
            .args([
                "--list-subs",
                "--force-ipv4",
                "--remote-components",
                "ejs:github",
                "--flat-playlist",
                &url,
            ])
            .output()
            .await
            .map_err(|e| Status::internal(format!("Failed to list subtitles: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        tracing::debug!("yt-dlp list-subs stdout: {}", stdout);
        tracing::debug!("yt-dlp list-subs stderr: {}", stderr);

        // Parse the output to extract subtitle info
        // Format is like:
        // Language    Name                                                          Formats
        // en          English, English, English...                                vtt, srt, ttml, srv3, srv2, srv1, json3, vtt
        let mut subtitles = Vec::new();
        let mut in_subtitle_section = false;

        for line in stdout.lines() {
            let line = line.trim();

            // Skip header and empty lines
            if line.is_empty() || line.starts_with("Language") || line.starts_with("---") {
                if line.starts_with("Language") {
                    in_subtitle_section = true;
                }
                continue;
            }

            if !in_subtitle_section {
                continue;
            }

            // Parse line like: "en             English, English...                                vtt, srt, ttml, srv3, srv2, srv1, json3, vtt"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let lang = parts[0].to_string();
                let is_auto = lang.ends_with("-orig") || lang.starts_with("(");
                let formats = parts[1..].join(" ");

                // Extract first format as default
                let first_format = formats
                    .split(',')
                    .next()
                    .unwrap_or("vtt")
                    .trim()
                    .to_string();

                subtitles.push(SubtitleInfo {
                    lang: lang.trim_end_matches("-orig").to_string(),
                    lang_name: formats.clone(),
                    ext: first_format,
                    is_auto,
                });
            }
        }

        Ok(Response::new(ListSubtitlesResponse { subtitles }))
    }

    async fn download_subtitles(
        &self,
        request: Request<DownloadSubtitlesRequest>,
    ) -> Result<Response<DownloadSubtitlesResponse>, Status> {
        let req = request.into_inner();
        let url = req.url;
        let lang = req.lang;
        let output_path = req.output_path;
        let format = if req.format.is_empty() {
            "vtt"
        } else {
            &req.format
        };

        tracing::info!(
            "Downloading subtitles for {} (lang: {}, format: {})",
            url,
            lang,
            format
        );

        // Create output directory if needed
        let output_path_buf = std::path::PathBuf::from(&output_path);
        let output_dir = if output_path_buf.is_dir() {
            output_path.clone()
        } else if let Some(parent) = output_path_buf.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            tokio::fs::create_dir_all(&parent_str)
                .await
                .map_err(|e| Status::internal(format!("Failed to create directory: {}", e)))?;
            parent_str
        } else {
            output_path.clone()
        };

        // Build yt-dlp command - output to directory with template
        let output_template = format!("{}/%(title)s.%(id)s", output_dir);
        let args = vec![
            "--write-subs",
            "--write-auto-subs",
            "--sub-lang",
            &lang,
            "--force-ipv4",
            "--remote-components",
            "ejs:github",
            "--skip-download",
            "--sub-format",
            format,
            "-o",
            &output_template,
            &url,
        ];

        let output = tokio::process::Command::new("yt-dlp")
            .args(&args)
            .output()
            .await
            .map_err(|e| Status::internal(format!("Failed to download subtitles: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("yt-dlp download-subs stderr: {}", stderr);

        if !output.status.success() {
            return Ok(Response::new(DownloadSubtitlesResponse {
                success: false,
                output_path: String::new(),
                file_size: 0,
                error: stderr.to_string(),
            }));
        }

        // Find the generated subtitle file
        // yt-dlp generates: title.id.lang.ext
        // First we need to extract video info to get title and id
        let video_info = extract_video_id(&url)
            .await
            .map_err(|e| Status::internal(format!("Failed to get video info: {}", e)))?;

        let subtitle_filename = format!("{}.{}.{}.{}", video_info.0, video_info.1, lang, format);
        let subtitle_path = format!("{}/{}", output_dir, subtitle_filename);

        // Try to find the actual file (yt-dlp might use different extensions)
        let actual_path = find_subtitle_file(&output_dir, &video_info.1, &lang)
            .await
            .unwrap_or(subtitle_path);

        // Get file size
        let metadata = tokio::fs::metadata(&actual_path)
            .await
            .map_err(|e| Status::internal(format!("Failed to stat output file: {}", e)))?;

        tracing::info!("Subtitles downloaded to: {}", actual_path);

        Ok(Response::new(DownloadSubtitlesResponse {
            success: true,
            output_path: actual_path,
            file_size: metadata.len() as i64,
            error: String::new(),
        }))
    }
}

async fn extract_video_id(url: &str) -> Result<(String, String), Status> {
    // Extract video ID from URL
    let video_id = if url.contains("youtu.be/") {
        url.split("youtu.be/")
            .nth(1)
            .and_then(|s| s.split('?').next())
            .unwrap_or(url)
            .to_string()
    } else if url.contains("watch?v=") {
        url.split("watch?v=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .unwrap_or(url)
            .to_string()
    } else {
        url.to_string()
    };

    // Get title from yt-dlp
    let output = tokio::process::Command::new("yt-dlp")
        .args([
            "--force-ipv4",
            "--remote-components",
            "ejs:github",
            "--get-title",
            "--flat-playlist",
            url,
        ])
        .output()
        .await
        .map_err(|e| Status::internal(format!("Failed to get title: {}", e)))?;

    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok((title, video_id))
}

async fn find_subtitle_file(dir: &str, video_id: &str, lang: &str) -> Option<String> {
    let mut read_dir = tokio::fs::read_dir(dir).await.ok()?;
    let lang_prefix = format!(".{}.", lang);

    while let Some(entry) = read_dir.next_entry().await.ok()? {
        let path = entry.path();
        let filename = path.file_name()?.to_string_lossy();

        // Look for files containing video_id and lang
        if filename.contains(video_id) && filename.contains(&lang_prefix) {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

impl Default for YtDlpService {
    fn default() -> Self {
        Self::new()
    }
}
