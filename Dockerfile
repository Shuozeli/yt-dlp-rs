# syntax=docker/dockerfile:1

# =============================================================================
# Stage 1: Build the yt-dlp-rs server
# =============================================================================
FROM --platform=$BUILDPLATFORM rust:1.94 AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /build

# Install cross-compilation dependencies if needed
RUN case "$TARGETPLATFORM" in \
    linux/amd64) echo "amd64" ;; \
    linux/arm64) apt-get update && apt-get install -y gcc-aarch64-linux-gnu ;; \
    esac || true

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY ytdlp-cli/Cargo.toml ytdlp-cli/
COPY ytdlp-proto/Cargo.toml ytdlp-proto/
COPY ytdlp-extractor/Cargo.toml ytdlp-extractor/
COPY ytdlp-extractors/Cargo.toml ytdlp-extractors/
COPY ytdlp-downloader/Cargo.toml ytdlp-downloader/
COPY ytdlp-net/Cargo.toml ytdlp-net/
COPY ytdlp-postproc/Cargo.toml ytdlp-postproc/
COPY ytdlp-server/Cargo.toml ytdlp-server/

# Create dummy source files for dependency compilation
RUN mkdir -p ytdlp-cli/src ytdlp-proto/src ytdlp-extractor/src \
    ytdlp-extractors/src ytdlp-downloader/src ytdlp-net/src \
    ytdlp-postproc/src ytdlp-server/src

# Build dependencies only (cached layer)
RUN cargo build --release --workspace

# Copy source code
COPY . .

# Build the server
RUN cargo build --release -p ytdlp-server

# =============================================================================
# Stage 2: Runtime image
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install yt-dlp, FFmpeg, and CA certificates
RUN apt-get update && apt-get install -y \
    yt-dlp \
    ffmpeg \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash appuser

WORKDIR /app

# Copy the built server binary
COPY --from=builder /build/target/release/ytdlp-server /app/ytdlp-server

# Copy config directory (if exists) and entrypoint
COPY --chown=appuser:appuser . /app/

# Switch to non-root user
USER appuser

EXPOSE 50051 50053

ENV RUST_LOG=info
ENV YT_DLP_SERVER_PORT=50053

ENTRYPOINT ["/app/ytdlp-server"]
