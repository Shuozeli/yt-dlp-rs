//! YouTube extractor for yt-dlp-rs
//!
//! Extracts video information from YouTube URLs.
//! Uses yt-dlp subprocess to handle signature decryption and format extraction.

use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use ytdlp_extractor::{Extractor, Format, Subtitle, SubtitleEntry, VideoInfo};
use ytdlp_net::{HttpClient, HttpOptions};

/// Cap on languages whose subtitle bodies we eagerly download per Extract
/// call. yt-dlp emits hundreds of auto-translated language variants for
/// most YouTube videos; pulling them all would balloon Extract latency.
/// Five covers the realistic cases (manual upload + a handful of common
/// auto-cap targets) without burning cycles on languages no caller will
/// read. Manual subs are preferred over auto-captions when both exist for
/// the same lang -- see `fetch_subtitles`.
const MAX_SUBTITLE_LANGS: usize = 5;

/// YouTube video extractor using yt-dlp subprocess
#[derive(Default)]
pub struct YoutubeExtractor;

impl YoutubeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract video ID from various YouTube URL formats
    pub fn extract_video_id(url: &str) -> Result<String> {
        let patterns = [
            r"youtube\.com/watch\?v=([^&]+)",
            r"youtu\.be/([^?]+)",
            r"youtube\.com/embed/([^?]+)",
            r"youtube\.com/v/([^?]+)",
            r"youtube\.com/shorts/([^?]+)",
            r"youtube\.com/live/([^?]+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(url) {
                    return Ok(caps[1].to_string());
                }
            }
        }
        anyhow::bail!("Could not extract video ID from URL: {}", url)
    }

    /// Run yt-dlp to extract video info as JSON
    async fn run_ytdlp(&self, url: &str) -> Result<YtDlpOutput> {
        // Use yt-dlp directly (installed via uv tool) with IPv4 and remote components
        let mut child = Command::new("yt-dlp")
            .args([
                "--dump-json",
                "--no-warnings",
                "--force-ipv4",
                "--remote-components",
                "ejs:github",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .context("Failed to spawn yt-dlp process")?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        {
            let stdout_pipe = child.stdout.take();
            let stderr_pipe = child.stderr.take();

            if let Some(mut s) = stdout_pipe {
                s.read_to_string(&mut stdout)
                    .await
                    .context("Failed to read stdout")?;
            }
            if let Some(mut s) = stderr_pipe {
                s.read_to_string(&mut stderr)
                    .await
                    .context("Failed to read stderr")?;
            }
        }

        let status = child.wait().await.context("Failed to wait for yt-dlp")?;

        if !status.success() {
            anyhow::bail!("yt-dlp failed: {}", stderr);
        }

        // Parse the JSON output
        let output: YtDlpOutput =
            serde_json::from_str(&stdout).context("Failed to parse yt-dlp JSON output")?;

        Ok(output)
    }
}

/// yt-dlp JSON output structure
#[derive(Debug, Deserialize)]
pub struct YtDlpOutput {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub uploader: Option<String>,
    pub uploader_url: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub formats: Vec<YtDlpFormat>,
    /// Manually uploaded subtitle tracks keyed by language code.
    #[serde(default)]
    pub subtitles: HashMap<String, Vec<YtDlpSubtitleVariant>>,
    /// Auto-generated caption tracks keyed by language code.
    #[serde(default)]
    pub automatic_captions: HashMap<String, Vec<YtDlpSubtitleVariant>>,
}

/// One downloadable subtitle variant from yt-dlp JSON. yt-dlp emits an
/// array per language listing the same caption track in several formats
/// (vtt / srv1 / srv3 / json3 / ttml / srt). We pick the VTT variant and
/// ignore the rest.
#[derive(Debug, Deserialize, Clone)]
pub struct YtDlpSubtitleVariant {
    pub url: String,
    pub ext: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct YtDlpFormat {
    pub format_id: String,
    pub ext: String,
    pub url: String,
    pub filesize: Option<u64>,
    pub format: Option<String>,
    pub format_note: Option<String>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub fps: Option<f64>,
    pub tbr: Option<f64>,
    pub resolution: Option<String>,
    #[serde(alias = "container")]
    pub container: Option<String>,
    #[serde(alias = "video_ext")]
    pub video_ext: Option<String>,
    #[serde(alias = "audio_ext")]
    pub audio_ext: Option<String>,
}

#[async_trait]
impl Extractor for YoutubeExtractor {
    fn name(&self) -> &str {
        "youtube"
    }

    fn supported_domains(&self) -> &[&str] {
        &[
            "youtube.com",
            "youtu.be",
            "www.youtube.com",
            "m.youtube.com",
        ]
    }

    async fn extract(&self, url: &str) -> Result<VideoInfo> {
        let video_id = Self::extract_video_id(url)?;

        tracing::info!("Extracting YouTube video info via yt-dlp: {}", video_id);

        // Use yt-dlp to get video info
        let output = self.run_ytdlp(url).await?;

        // Convert yt-dlp formats to our format
        let formats: Vec<Format> = output
            .formats
            .into_iter()
            .filter(|f| !f.url.is_empty())
            .map(|f| {
                let resolution = f.resolution.or(f.format_note);
                let tbr = f.tbr.map(|t| t / 1000.0); // Convert to kbps

                Format {
                    format_id: f.format_id,
                    ext: f.ext,
                    resolution,
                    filesize: f.filesize,
                    vcodec: f.vcodec.filter(|c| c != "none"),
                    acodec: f.acodec.filter(|c| c != "none"),
                    fps: f.fps.map(|f| f as u32),
                    tbr,
                    protocol: "https".to_string(),
                    url: Some(f.url),
                }
            })
            .collect();

        // Build thumbnail URL if not provided
        let thumbnail = output.thumbnail.or_else(|| {
            Some(format!(
                "https://i.ytimg.com/vi/{}/maxresdefault.jpg",
                video_id
            ))
        });

        let duration = output
            .duration
            .map(|d| std::time::Duration::from_secs(d as u64));

        // yt-dlp's --dump-json gives us subtitle URLs but not bodies. Fetch
        // and parse the VTT for up to MAX_SUBTITLE_LANGS languages so the
        // gRPC client gets text inline instead of having to round-trip
        // through DownloadSubtitles (which writes to the server's local
        // disk and is unreachable cross-pod).
        let subtitles = fetch_subtitles(output.subtitles, output.automatic_captions).await;

        Ok(VideoInfo {
            id: output.id,
            title: output.title,
            description: output.description,
            uploader: output.uploader,
            uploader_url: output.uploader_url,
            duration,
            thumbnail: thumbnail.and_then(|t| url::Url::parse(&t).ok()),
            formats,
            subtitles,
            thumbnails: vec![],
            metadata: HashMap::new(),
        })
    }
}

/// Fetch and VTT-parse subtitle bodies for the languages yt-dlp listed.
///
/// Manual subtitles take precedence; we only fall back to auto-captions
/// for languages with no manual track. Total fetches are capped at
/// MAX_SUBTITLE_LANGS, and individual failures (network error, non-2xx,
/// empty body) are logged at warn and skipped -- a single bad language
/// must not poison the whole Extract response.
///
/// Returns a HashMap<lang, Vec<Subtitle>> matching VideoInfo's shape.
/// The Vec usually holds one entry per language, but the schema allows
/// multiples (e.g. manual + auto for the same lang) and we honour that.
async fn fetch_subtitles(
    manual: HashMap<String, Vec<YtDlpSubtitleVariant>>,
    auto: HashMap<String, Vec<YtDlpSubtitleVariant>>,
) -> HashMap<String, Vec<Subtitle>> {
    let http = match HttpClient::new(HttpOptions::default()) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "subtitle http client init failed; returning empty subs");
            return HashMap::new();
        }
    };

    // Build the merged lang queue: manual first, then auto for any lang
    // not already covered. Caps at MAX_SUBTITLE_LANGS to bound latency.
    let manual_langs: HashSet<String> = manual.keys().cloned().collect();
    let mut queue: Vec<(String, Vec<YtDlpSubtitleVariant>, bool)> = Vec::new();
    for (lang, variants) in manual {
        queue.push((lang, variants, false /* is_auto */));
        if queue.len() >= MAX_SUBTITLE_LANGS {
            break;
        }
    }
    if queue.len() < MAX_SUBTITLE_LANGS {
        for (lang, variants) in auto {
            if manual_langs.contains(&lang) {
                continue;
            }
            queue.push((lang, variants, true));
            if queue.len() >= MAX_SUBTITLE_LANGS {
                break;
            }
        }
    }

    if queue.is_empty() {
        return HashMap::new();
    }

    // Fan out: one task per language, joined at the end.
    let mut handles = Vec::with_capacity(queue.len());
    for (lang, variants, _is_auto) in queue {
        let Some(variant) = pick_vtt(&variants) else {
            tracing::warn!(lang = %lang, "no vtt variant available; skipping");
            continue;
        };
        let url = variant.url.clone();
        let ext = variant.ext.clone();
        let http = http.clone();
        let lang_for_task = lang.clone();
        handles.push(tokio::spawn(async move {
            let body = match http.get(&url).await {
                Ok(resp) if resp.status == 200 => resp.body,
                Ok(resp) => {
                    tracing::warn!(lang = %lang_for_task, status = resp.status, "subtitle fetch non-200; skipping");
                    return None;
                }
                Err(e) => {
                    tracing::warn!(lang = %lang_for_task, error = %e, "subtitle fetch error; skipping");
                    return None;
                }
            };
            let text = String::from_utf8_lossy(&body);
            let entries = parse_vtt(&text);
            if entries.is_empty() {
                tracing::warn!(lang = %lang_for_task, "subtitle parsed 0 entries; skipping");
                return None;
            }
            Some((
                lang_for_task.clone(),
                Subtitle {
                    lang: lang_for_task,
                    ext,
                    entries,
                },
            ))
        }));
    }

    let mut result: HashMap<String, Vec<Subtitle>> = HashMap::new();
    for h in handles {
        if let Ok(Some((lang, sub))) = h.await {
            result.entry(lang).or_default().push(sub);
        }
    }
    result
}

/// Prefer the VTT variant; fall back to the first variant if VTT is
/// missing (rare). VTT parses cleanly; srv3/json3 have format-specific
/// quirks we don't want to handle today.
fn pick_vtt(variants: &[YtDlpSubtitleVariant]) -> Option<&YtDlpSubtitleVariant> {
    variants
        .iter()
        .find(|v| v.ext.eq_ignore_ascii_case("vtt"))
        .or_else(|| variants.first())
}

/// Minimal WEBVTT parser. Walks the file line by line, picks out cue
/// timing lines of the form
///
///   00:00:01.000 --> 00:00:03.000  [optional positioning info]
///
/// and treats the lines that follow (until the next blank line) as the
/// cue text. Multi-line cue text is joined with " " so downstream
/// transcript builders can concatenate cues directly.
///
/// Not a complete WEBVTT implementation: we ignore NOTE blocks, STYLE
/// blocks, REGION blocks, cue identifiers, and inline positioning tags.
/// For YouTube auto-captions and manual-upload VTTs that's enough.
fn parse_vtt(content: &str) -> Vec<SubtitleEntry> {
    let mut entries = Vec::new();
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        let Some((start, end)) = parse_vtt_timing(line) else {
            continue;
        };
        let mut text_lines: Vec<String> = Vec::new();
        for text_line in lines.by_ref() {
            if text_line.trim().is_empty() {
                break;
            }
            text_lines.push(strip_vtt_inline_tags(text_line));
        }
        let text = text_lines.join(" ").trim().to_string();
        if !text.is_empty() {
            entries.push(SubtitleEntry { start, end, text });
        }
    }
    entries
}

fn parse_vtt_timing(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let idx = trimmed.find(" --> ")?;
    let start = trimmed[..idx].trim().to_string();
    let end_part = &trimmed[idx + 5..];
    // After `-->` an optional cue settings string can follow the end
    // timestamp (e.g. `00:00:03.000 line:90% align:start`); take only
    // the first whitespace-delimited token.
    let end = end_part.split_whitespace().next().unwrap_or("").to_string();
    if start.is_empty() || end.is_empty() {
        return None;
    }
    Some((start, end))
}

/// Strip WEBVTT inline tags like `<c.colorE5E5E5>foo</c>` or `<00:00:01.000>`
/// so the resulting transcript text reads as plain prose. Conservative:
/// only strips bracketed tags, not entity references.
fn strip_vtt_inline_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '<' {
            // Skip until matching '>'.
            for tc in chars.by_ref() {
                if tc == '>' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        let test_cases = vec![
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://youtu.be/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/embed/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/shorts/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
        ];

        for (url, expected_id) in test_cases {
            let result = YoutubeExtractor::extract_video_id(url);
            assert!(result.is_ok(), "Failed for URL: {}", url);
            assert_eq!(result.unwrap(), expected_id, "Mismatch for URL: {}", url);
        }
    }

    #[test]
    fn test_parse_vtt_timing_basic() {
        let (s, e) = parse_vtt_timing("00:00:01.000 --> 00:00:03.500").unwrap();
        assert_eq!(s, "00:00:01.000");
        assert_eq!(e, "00:00:03.500");
    }

    #[test]
    fn test_parse_vtt_timing_with_settings() {
        // YouTube auto-captions emit cue settings after the end timestamp.
        let (s, e) =
            parse_vtt_timing("00:00:01.000 --> 00:00:03.500 line:90% align:start").unwrap();
        assert_eq!(s, "00:00:01.000");
        assert_eq!(e, "00:00:03.500");
    }

    #[test]
    fn test_parse_vtt_timing_rejects_garbage() {
        assert!(parse_vtt_timing("WEBVTT").is_none());
        assert!(parse_vtt_timing("").is_none());
        assert!(parse_vtt_timing("00:00:01.000").is_none());
    }

    #[test]
    fn test_parse_vtt_full_document() {
        let vtt = "WEBVTT

00:00:01.000 --> 00:00:03.000
Hello world

00:00:04.000 --> 00:00:06.000 line:90%
Multi
line cue
";
        let entries = parse_vtt(vtt);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].start, "00:00:01.000");
        assert_eq!(entries[0].end, "00:00:03.000");
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[1].text, "Multi line cue");
    }

    #[test]
    fn test_parse_vtt_strips_inline_tags() {
        let vtt = "WEBVTT

00:00:01.000 --> 00:00:03.000
<c.colorE5E5E5>Spoken</c> word
";
        let entries = parse_vtt(vtt);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Spoken word");
    }

    #[test]
    fn test_pick_vtt_prefers_vtt_variant() {
        let variants = vec![
            YtDlpSubtitleVariant {
                url: "https://example/srv3".into(),
                ext: "srv3".into(),
                name: None,
            },
            YtDlpSubtitleVariant {
                url: "https://example/vtt".into(),
                ext: "vtt".into(),
                name: None,
            },
        ];
        let pick = pick_vtt(&variants).unwrap();
        assert_eq!(pick.ext, "vtt");
    }

    #[test]
    fn test_pick_vtt_falls_back_when_no_vtt() {
        let variants = vec![YtDlpSubtitleVariant {
            url: "https://example/srt".into(),
            ext: "srt".into(),
            name: None,
        }];
        let pick = pick_vtt(&variants).unwrap();
        assert_eq!(pick.ext, "srt");
    }
}
