//! gRPC server for yt-dlp functionality

use anyhow::Result;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    tracing::info!("Starting yt-dlp gRPC server");

    let addr = "[::1]:50051".parse()?;
    tracing::info!("Server listening on {}", addr);

    Server::builder()
        .add_service()
        .serve(addr)
        .await?;

    Ok(())
}
