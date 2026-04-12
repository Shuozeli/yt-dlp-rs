//! CLI client for yt-dlp gRPC server

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ytdlp")]
#[command(version = "0.1.0")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Extract video information
    Info {
        #[arg(short, long)]
        url: String,
    },
    /// Download a video
    Download {
        #[arg(short, long)]
        url: String,
        #[arg(short, long, default_value = "best")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List supported sites
    Sites,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Info { url } => {
            println!("Extracting info for: {}", url);
        }
        Command::Download { url, format, output } => {
            println!("Downloading {} (format: {}) to {:?}", url, format, output);
        }
        Command::Sites => {
            println!("Listing supported sites...");
        }
    }

    Ok(())
}
