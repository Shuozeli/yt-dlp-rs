//! CLI client for yt-dlp gRPC server

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

mod client;
mod config;
mod output_template;
mod progress;

use client::{Client, DownloadOpts};
use config::Config;
use output_template::OutputTemplate;

// ============================================================================
// CLI Structure
// ============================================================================

#[derive(Parser, Debug)]
#[command(
    name = "ytdlp",
    version = "0.1.0",
    about = "yt-dlp-rs CLI client for video downloading and information extraction"
)]
struct Cli {
    /// Server address (default: http://[::1]:50053)
    #[arg(short, long)]
    server: Option<String>,

    #[command(subcommand)]
    command: Commands,

    /// Increase verbosity (can be repeated: -v, -vv, -vvv, -vvvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract video information
    Info {
        /// Video URL to extract info from
        #[arg(short, long)]
        url: String,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// List available formats
        #[arg(short = 'F', long)]
        list_formats: bool,
    },

    /// Download a video
    Download {
        /// Video URL to download
        #[arg(short, long)]
        url: String,

        /// Format selector (e.g., "best", "bestvideo+bestaudio", "22")
        #[arg(short, long, default_value = "best")]
        format: String,

        /// Output file path (when using single file, use --output-template for templates)
        #[arg(short, long)]
        output: Option<String>,

        /// Output filename template
        #[arg(long)]
        output_template: Option<String>,

        /// Number of retries
        #[arg(short = 'R', long)]
        retries: Option<u32>,

        /// Rate limit (e.g., "1M" for 1 MB/s)
        #[arg(short = 'r', long)]
        rate_limit: Option<String>,

        /// Proxy URL
        #[arg(long)]
        proxy: Option<String>,

        /// User agent string
        #[arg(short = 'U', long)]
        user_agent: Option<String>,

        /// Continue partial downloads
        #[arg(short = 'c', long)]
        continue_download: bool,
    },

    /// List supported sites
    Sites {
        /// Search for sites matching pattern
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Show current configuration
    Config {
        /// Show config file location
        #[arg(long)]
        show_path: bool,
    },

    /// Check server health
    Health,

    /// List available subtitles/transcripts for a video
    Transcript {
        /// Video URL
        #[arg(short, long)]
        url: String,

        /// Language code (default: en)
        #[arg(short, long, default_value = "en")]
        lang: String,
    },

    /// Download subtitles/transcripts for a video
    DownloadSubs {
        /// Video URL
        #[arg(short, long)]
        url: String,

        /// Language code (default: en)
        #[arg(short, long, default_value = "en")]
        lang: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Output format: vtt, srt, ttml, json3 (default: vtt)
        #[arg(short, long, default_value = "vtt")]
        format: String,
    },
}

// ============================================================================
// Logging Setup
// ============================================================================

fn setup_logging(verbose: u8) {
    let env_filter = match verbose {
        0 => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        1 => EnvFilter::new("warn"),
        2 => EnvFilter::new("info"),
        3 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    fmt()
        .with_env_filter(env_filter)
        .with_target(verbose >= 3)
        .with_thread_ids(verbose >= 4)
        .with_file(verbose >= 4)
        .with_line_number(verbose >= 4)
        .init();
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    // Load global config
    let global_config = Config::load()?.unwrap_or_default();

    // Connect to server
    let server_addr = cli.server.as_deref().unwrap_or("");
    let mut client = Client::connect(server_addr).await?;

    match cli.command {
        Commands::Info {
            url,
            json,
            list_formats,
        } => {
            info_cmd(&mut client, &url, json, list_formats).await?;
        }

        Commands::Download {
            url,
            format,
            output,
            output_template,
            retries,
            rate_limit,
            proxy,
            user_agent,
            continue_download,
        } => {
            let opts = DownloadOpts {
                retries: retries.unwrap_or(global_config.retries),
                rate_limit: rate_limit.or(global_config.rate_limit),
                proxy: proxy.or(global_config.proxy),
                user_agent: user_agent.or(global_config.user_agent),
                continue_download,
            };
            download_cmd(
                &mut client,
                &url,
                &format,
                output.as_deref(),
                output_template.as_deref(),
                opts,
            )
            .await?;
        }

        Commands::Sites { search } => {
            sites_cmd(&mut client, search.as_deref()).await?;
        }

        Commands::Config { show_path } => {
            config_cmd(show_path)?;
        }

        Commands::Health => {
            health_cmd(&mut client).await?;
        }

        Commands::Transcript { url, lang } => {
            transcript_cmd(&mut client, &url, &lang).await?;
        }

        Commands::DownloadSubs {
            url,
            lang,
            output,
            format,
        } => {
            download_subs_cmd(&mut client, &url, &lang, &output, &format).await?;
        }
    }

    Ok(())
}

// ============================================================================
// Info Command
// ============================================================================

async fn info_cmd(client: &mut Client, url: &str, json: bool, list_formats: bool) -> Result<()> {
    tracing::info!("Extracting info for: {}", url);

    let video = client.extract(url).await?;

    if json {
        // Build a manual JSON representation since proto doesn't derive Serialize
        let json_value = serde_json::json!({
            "id": video.id,
            "title": video.title,
            "description": video.description,
            "uploader": video.uploader,
            "uploader_url": video.uploader_url,
            "duration": video.duration,
            "metadata": video.metadata,
            "formats": video.formats.iter().map(|f| serde_json::json!({
                "format_id": f.format_id,
                "ext": f.ext,
                "resolution": f.resolution,
                "filesize": f.filesize,
            })).collect::<Vec<_>>(),
            "subtitles": video.subtitles.iter().map(|s| serde_json::json!({
                "lang": s.lang,
                "ext": s.ext,
            })).collect::<Vec<_>>(),
            "thumbnails": video.thumbnails.iter().map(|t| serde_json::json!({
                "url": t.url,
                "width": t.width,
                "height": t.height,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json_value)?);
    } else {
        println!("Video URL: {}", url);
        println!("Title: {}", video.title);
        println!("ID: {}", video.id);
        if !video.uploader.is_empty() {
            println!("Uploader: {}", video.uploader);
        }
        if video.duration > 0 {
            let secs = video.duration % 60;
            let mins = (video.duration / 60) % 60;
            let hours = video.duration / 3600;
            if hours > 0 {
                println!("Duration: {}:{:02}:{:02}", hours, mins, secs);
            } else {
                println!("Duration: {}:{:02}", mins, secs);
            }
        }
        if !video.description.is_empty() {
            let desc = if video.description.len() > 200 {
                format!("{}...", &video.description[..200])
            } else {
                video.description.clone()
            };
            println!("Description: {}", desc);
        }

        if !video.metadata.is_empty() {
            println!("\nMetadata:");
            for (key, value) in &video.metadata {
                println!("  {}: {}", key, value);
            }
        }

        if !video.thumbnails.is_empty() {
            println!("\nThumbnails ({}):", video.thumbnails.len());
            for thumb in &video.thumbnails {
                println!("  {}x{} - {}", thumb.width, thumb.height, thumb.url);
            }
        }

        if !video.subtitles.is_empty() {
            println!("\nSubtitles:");
            for sub in &video.subtitles {
                println!("  {} ({})", sub.lang, sub.ext);
            }
        }

        if list_formats && !video.formats.is_empty() {
            println!("\nAvailable formats:");
            for fmt in &video.formats {
                let res = if fmt.resolution.is_empty() {
                    "unknown"
                } else {
                    fmt.resolution.as_str()
                };
                println!("  {} - {} ({})", fmt.format_id, res, fmt.ext);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Download Command
// ============================================================================

async fn download_cmd(
    client: &mut Client,
    url: &str,
    format: &str,
    output: Option<&str>,
    output_template: Option<&str>,
    opts: DownloadOpts,
) -> Result<()> {
    tracing::info!("Download requested: {} (format: {})", url, format);

    // First extract info to get the format details
    let video = client.extract(url).await?;

    // Find the matching format and get its download URL
    let download_url = video
        .formats
        .iter()
        .find(|f| f.format_id == format)
        .and_then(|f| {
            if f.url.is_empty() {
                None
            } else {
                Some(f.url.clone())
            }
        });

    // Determine output path
    let output_path = output.map(PathBuf::from).unwrap_or_else(|| {
        let template = output_template.unwrap_or("%(title)s-%(id)s.%(ext)s");
        let _tmpl = OutputTemplate::new(template);
        PathBuf::from(format!(
            "{}.{}",
            video.id,
            video
                .formats
                .first()
                .map(|f| f.ext.as_str())
                .unwrap_or("mp4")
        ))
    });

    tracing::info!("Output path: {:?}", output_path);

    // Perform the download (pass download_url if available)
    client
        .download(
            url,
            format,
            output_path.to_str().unwrap_or("video.mp4"),
            download_url.as_deref(),
            opts,
        )
        .await?;

    Ok(())
}

// ============================================================================
// Sites Command
// ============================================================================

async fn sites_cmd(client: &mut Client, search: Option<&str>) -> Result<()> {
    tracing::info!("Listing supported sites...");

    let sites = client.list_supported_sites().await?;

    if let Some(pattern) = search {
        let pattern_lower = pattern.to_lowercase();
        let filtered: Vec<_> = sites
            .iter()
            .filter(|s| s.to_lowercase().contains(&pattern_lower))
            .collect();

        if filtered.is_empty() {
            println!("No sites matching '{}' found.", pattern);
        } else {
            println!("Sites matching '{}':", pattern);
            for site in filtered {
                println!("  - {}", site);
            }
        }
    } else {
        println!("Supported sites ({} total):", sites.len());
        for site in sites {
            println!("  - {}", site);
        }
    }

    Ok(())
}

// ============================================================================
// Health Command
// ============================================================================

async fn health_cmd(client: &mut Client) -> Result<()> {
    let health = client.health().await?;

    if health.healthy {
        println!("Server is healthy (version: {})", health.version);
    } else {
        println!("Server is unhealthy");
    }

    Ok(())
}

// ============================================================================
// Config Command
// ============================================================================

fn config_cmd(show_path: bool) -> Result<()> {
    if show_path {
        let path = Config::config_path()?;
        println!("Config file: {:?}", path);

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            println!("\nContents:");
            println!("{}", content);
        } else {
            println!("(Config file does not exist yet)");
        }
        return Ok(());
    }

    // Show effective config
    let config = Config::load()?.unwrap_or_default();
    println!("Current configuration:");
    println!("  output_template: {}", config.output_template);
    println!("  retries: {}", config.retries);
    println!("  rate_limit: {:?}", config.rate_limit);
    println!("  proxy: {:?}", config.proxy);
    println!("  user_agent: {:?}", config.user_agent);

    Ok(())
}

// ============================================================================
// Transcript Command
// ============================================================================

async fn transcript_cmd(client: &mut Client, url: &str, lang: &str) -> Result<()> {
    tracing::info!("Listing subtitles for: {} (lang: {})", url, lang);

    let subtitles = client.list_subtitles(url, lang).await?;

    if subtitles.is_empty() {
        println!("No subtitles found for this video.");
        return Ok(());
    }

    println!("Available subtitles:");
    println!("{:<10} {:<40} {:<10}", "Language", "Name", "Format");
    println!("{}", "-".repeat(60));

    for sub in &subtitles {
        let name = if sub.lang_name.len() > 38 {
            format!("{}...", &sub.lang_name[..35])
        } else {
            sub.lang_name.clone()
        };
        let auto_tag = if sub.is_auto { " (auto)" } else { "" };
        println!("{:<10} {:<40} {:<10}{}", sub.lang, name, sub.ext, auto_tag);
    }

    Ok(())
}

// ============================================================================
// Download Subs Command
// ============================================================================

async fn download_subs_cmd(
    client: &mut Client,
    url: &str,
    lang: &str,
    output: &str,
    format: &str,
) -> Result<()> {
    tracing::info!(
        "Downloading subtitles for: {} (lang: {}, format: {})",
        url,
        lang,
        format
    );

    let result = client.download_subtitles(url, lang, output, format).await?;

    if result.success {
        println!("Subtitles downloaded successfully!");
        println!(
            "Output: {} ({} bytes)",
            result.output_path, result.file_size
        );
    } else {
        println!("Failed to download subtitles: {}", result.error);
    }

    Ok(())
}
