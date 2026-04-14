//! gRPC server for yt-dlp functionality

mod service;

use anyhow::Result;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use service::YtDlpService;
use ytdlp_proto::proto::yt_dlp_server::YtDlpServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting yt-dlp gRPC server");

    let addr: SocketAddr = "[::]:50053".parse()?;
    tracing::info!("Server listening on {}", addr);

    let service = YtDlpService::new();

    Server::builder()
        .add_service(YtDlpServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
