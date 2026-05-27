use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Proxy};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use super::downloader::{DownloadOptions, DownloadResult, Downloader, Progress};

/// Default HTTP range-chunk size. Matches yt-dlp's `http_chunk_size` default.
///
/// YouTube throttles each anonymous connection to roughly playback rate after
/// the first few MB, so a single open-ended GET crawls (and eventually the
/// connection is dropped, surfacing as `failed to read chunk`). Requesting the
/// file as a sequence of bounded `Range: bytes=start-end` windows defeats this:
/// each fresh ranged request gets a new fast burst before the throttle engages,
/// sustaining tens of MiB/s. Empirically a 32 MiB audio that took 17 min (then
/// failed) over a single stream completes in ~4 s with 10 MiB windows.
const DEFAULT_CHUNK_SIZE: u64 = 10 * 1024 * 1024;

pub struct HttpDownloader {
    client: Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        Self::with_options(&DownloadOptions::default()).expect("failed to create HTTP client")
    }

    pub fn with_options(options: &DownloadOptions) -> anyhow::Result<Self> {
        // Per-request timeout. With range chunking each request transfers at
        // most one window (default 10 MiB) at burst speed, so this is a
        // generous ceiling for a single window, not the whole file.
        let mut builder = Client::builder().timeout(Duration::from_secs(120));

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

    /// Fetch one byte window `[start, end]` (inclusive) into memory, retrying
    /// transient failures. Returns the body bytes, the parsed total file size
    /// (from `Content-Range` when present), and the HTTP status code.
    ///
    /// Buffering the whole window means a mid-window failure simply discards
    /// the buffer and retries -- it can never write a torn fragment to disk.
    async fn fetch_window(
        &self,
        url: &str,
        start: u64,
        end: u64,
        max_retries: u32,
    ) -> anyhow::Result<WindowResult> {
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=max_retries.max(1) {
            match self.try_fetch_window(url, start, end).await {
                Ok(r) => return Ok(r),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < max_retries.max(1) {
                        tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("window fetch failed")))
    }

    async fn try_fetch_window(
        &self,
        url: &str,
        start: u64,
        end: u64,
    ) -> anyhow::Result<WindowResult> {
        let response = self
            .client
            .get(url)
            .header("Range", format!("bytes={}-{}", start, end))
            .send()
            .await
            .context("failed to send range request")?;

        let status = response.status().as_u16();

        // 416 = Range Not Satisfiable: we asked past EOF, i.e. nothing more.
        if status == 416 {
            return Ok(WindowResult {
                bytes: Vec::new(),
                total: None,
                status,
            });
        }
        if status == 200 {
            // Server ignored the Range header; this body is the entire file.
            let total = response.content_length();
            let bytes = response
                .bytes()
                .await
                .context("failed to read full body")?
                .to_vec();
            return Ok(WindowResult {
                bytes,
                total,
                status,
            });
        }
        if status != 206 {
            anyhow::bail!("HTTP request failed with status: {}", status);
        }

        let total = response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_content_range_total);
        let bytes = response
            .bytes()
            .await
            .context("failed to read chunk")?
            .to_vec();
        Ok(WindowResult {
            bytes,
            total,
            status,
        })
    }
}

/// Outcome of fetching one range window.
struct WindowResult {
    bytes: Vec<u8>,
    total: Option<u64>,
    status: u16,
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
        options: DownloadOptions,
        progress: impl Fn(Progress) + Send,
    ) -> anyhow::Result<DownloadResult> {
        let start = Instant::now();
        let chunk_size = options
            .part_size
            .filter(|s| *s > 0)
            .unwrap_or(DEFAULT_CHUNK_SIZE);
        let max_retries = options.retries.max(1);

        // Resume support: continue from the end of an existing partial file.
        let existing_size = if dest.exists() {
            tokio::fs::metadata(dest).await.map(|m| m.len()).ok()
        } else {
            None
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
        // Seek to the resume offset; subsequent writes are sequential.
        if downloaded > 0 {
            file.seek(std::io::SeekFrom::Start(downloaded))
                .await
                .context("failed to seek to resume offset")?;
        }

        let mut total: Option<u64> = None;
        let mut written_this_session = 0u64;

        loop {
            if let Some(t) = total {
                if downloaded >= t {
                    break;
                }
            }

            let range_start = downloaded;
            let range_end = match total {
                Some(t) => (range_start + chunk_size - 1).min(t.saturating_sub(1)),
                None => range_start + chunk_size - 1,
            };

            let window = self
                .fetch_window(url, range_start, range_end, max_retries)
                .await?;

            // Range Not Satisfiable -> we are already at EOF.
            if window.status == 416 {
                break;
            }

            // Server ignored Range (200): the body is the whole file. Restart
            // from byte 0 (discarding any range-based partial) and stream it.
            if window.status == 200 {
                if range_start != 0 {
                    file.seek(std::io::SeekFrom::Start(0))
                        .await
                        .context("failed to rewind for non-range download")?;
                    downloaded = 0;
                    written_this_session = 0;
                }
                file.write_all(&window.bytes)
                    .await
                    .context("failed to write body")?;
                downloaded += window.bytes.len() as u64;
                written_this_session += window.bytes.len() as u64;
                total = Some(downloaded);
                emit_progress(&progress, downloaded, total, written_this_session, &start);
                break;
            }

            // 206 Partial Content.
            if total.is_none() {
                total = window.total;
            }
            if window.bytes.is_empty() {
                // No more data despite a 206; avoid an infinite loop.
                break;
            }

            let n = window.bytes.len() as u64;
            file.write_all(&window.bytes)
                .await
                .context("failed to write chunk")?;
            downloaded += n;
            written_this_session += n;

            emit_progress(&progress, downloaded, total, written_this_session, &start);

            // If the total is unknown and the server returned a short window,
            // we have reached the end.
            if total.is_none() && n < (range_end - range_start + 1) {
                break;
            }
        }

        // Trim any stale tail (e.g. from a shrunk re-download) so the file size
        // is exactly the number of bytes we accounted for.
        file.set_len(downloaded)
            .await
            .context("failed to set final file length")?;
        file.flush().await.context("failed to flush file")?;

        Ok(DownloadResult {
            output_path: dest.to_path_buf(),
            total_bytes: written_this_session,
        })
    }
}

/// Emit a progress update derived from the running download.
fn emit_progress(
    progress: &(impl Fn(Progress) + Send),
    downloaded: u64,
    total: Option<u64>,
    written_this_session: u64,
    start: &Instant,
) {
    let elapsed = start.elapsed().as_secs_f64();
    let speed = if elapsed > 0.0 {
        written_this_session as f64 / elapsed
    } else {
        0.0
    };
    let eta = match (total, speed > 0.0) {
        (Some(t), true) => Some(t.saturating_sub(downloaded) as f64 / speed),
        _ => None,
    };
    progress(Progress {
        downloaded_bytes: downloaded,
        total_bytes: total,
        speed,
        eta_seconds: eta,
    });
}

/// Parse the total size out of a `Content-Range: bytes start-end/total` header
/// value. Returns `None` when the total is unknown (`*`) or unparseable.
fn parse_content_range_total(header_value: &str) -> Option<u64> {
    let total = header_value.rsplit('/').next()?.trim();
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn parse_content_range_total_cases() {
        assert_eq!(parse_content_range_total("bytes 0-9/100"), Some(100));
        assert_eq!(
            parse_content_range_total("bytes 1024-2047/33339349"),
            Some(33339349)
        );
        assert_eq!(parse_content_range_total("bytes 0-9/*"), None);
        assert_eq!(parse_content_range_total("garbage"), None);
    }

    /// Minimal HTTP/1.1 origin used as a Fake (not a mock): it actually serves
    /// `body` and honors a single `Range: bytes=a-b` request, replying 206 with
    /// a `Content-Range`. When `support_range` is false it ignores Range and
    /// returns the full body with 200 -- exercising the fallback path.
    async fn spawn_fake_origin(body: Vec<u8>, support_range: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = match listener.accept().await {
                    Ok(c) => c,
                    Err(_) => break,
                };
                let body = body.clone();
                tokio::spawn(async move {
                    // Read request headers (until CRLFCRLF).
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 1024];
                    loop {
                        match sock.read(&mut tmp).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                buf.extend_from_slice(&tmp[..n]);
                                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                    break;
                                }
                            }
                        }
                    }
                    let req = String::from_utf8_lossy(&buf);
                    let range = req.lines().find_map(|l| {
                        let l = l.trim();
                        if l.to_ascii_lowercase().starts_with("range:") {
                            l.split('=').nth(1).map(|s| s.trim().to_string())
                        } else {
                            None
                        }
                    });
                    let total = body.len();
                    let resp: Vec<u8> = if let Some(r) = range.filter(|_| support_range) {
                        let mut parts = r.split('-');
                        let a: usize = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
                        let b: usize = parts
                            .next()
                            .and_then(|x| x.trim().parse().ok())
                            .unwrap_or(total.saturating_sub(1))
                            .min(total.saturating_sub(1));
                        if a >= total {
                            format!(
                                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                                total
                            )
                            .into_bytes()
                        } else {
                            let slice = &body[a..=b];
                            let mut h = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                a, b, total, slice.len()
                            )
                            .into_bytes();
                            h.extend_from_slice(slice);
                            h
                        }
                    } else {
                        let mut h = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            total
                        )
                        .into_bytes();
                        h.extend_from_slice(&body);
                        h
                    };
                    let _ = sock.write_all(&resp).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        format!("http://{}/file", addr)
    }

    fn sample_body(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[tokio::test]
    async fn chunked_download_reassembles_file() {
        let body = sample_body(25 * 1024); // 25 KiB
        let url = spawn_fake_origin(body.clone(), true).await;
        let dl = HttpDownloader::new();
        let dest = std::env::temp_dir().join(format!("ytdlp_chunk_{}.bin", std::process::id()));
        let _ = std::fs::remove_file(&dest);
        // 4 KiB windows force ~7 ranged requests.
        let opts = DownloadOptions {
            part_size: Some(4096),
            ..Default::default()
        };
        let res = dl.download(&url, "t", &dest, opts, |_| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        assert_eq!(res.total_bytes, body.len() as u64);
        let _ = std::fs::remove_file(&dest);
    }

    #[tokio::test]
    async fn falls_back_to_full_body_when_range_ignored() {
        let body = sample_body(10 * 1024);
        let url = spawn_fake_origin(body.clone(), false).await;
        let dl = HttpDownloader::new();
        let dest = std::env::temp_dir().join(format!("ytdlp_full_{}.bin", std::process::id()));
        let _ = std::fs::remove_file(&dest);
        let opts = DownloadOptions {
            part_size: Some(4096),
            ..Default::default()
        };
        dl.download(&url, "t", &dest, opts, |_| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        let _ = std::fs::remove_file(&dest);
    }
}
