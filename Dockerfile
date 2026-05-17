# syntax=docker/dockerfile:1

# =============================================================================
# Stage 1: Build the yt-dlp-rs server
# =============================================================================
# Pin to bookworm to match the runtime stage's debian:bookworm-slim
# (glibc 2.36). The unpinned `rust:1.94` tag follows debian:trixie which
# links the server against glibc 2.39, producing
# `/lib/x86_64-linux-gnu/libc.so.6: version 'GLIBC_2.39' not found` at
# container start on the runtime image.
FROM --platform=$BUILDPLATFORM rust:1.94-bookworm AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /build

# Install cross-compilation dependencies if needed
RUN case "$TARGETPLATFORM" in \
    linux/amd64) echo "amd64" ;; \
    linux/arm64) apt-get update && apt-get install -y gcc-aarch64-linux-gnu ;; \
    esac || true

# prost-build's build.rs needs `protoc` to compile the .proto. CI installs
# protobuf-compiler explicitly; the Dockerfile previously relied on a
# leftover binary that no longer ships in rust:1.94.
RUN apt-get update && apt-get install -y --no-install-recommends \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy entire source (target/ excluded via .dockerignore). The previous
# Dockerfile attempted a manifest-only dep-cache trick, but its stub
# build cached our own crates' .rlibs with empty contents, which then
# poisoned the real build's incremental detection and produced
# `unresolved imports ytdlp_extractor::Extractor` at link time. Simpler
# and reliable: just compile the workspace once from real source.
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

# Copy the built server binary. The original Dockerfile also did a
# `COPY --chown=appuser:appuser . /app/` here, but the source tree
# contains a `ytdlp-server/` directory that collides with the
# `/app/ytdlp-server` binary above, producing
# `cannot copy to non-directory` during the runtime stage. The binary
# is the only artifact the entrypoint needs; the source copy was dead
# weight.
COPY --from=builder /build/target/release/ytdlp-server /app/ytdlp-server
RUN chown appuser:appuser /app/ytdlp-server

# Switch to non-root user
USER appuser

EXPOSE 50051 50053

ENV RUST_LOG=info
ENV YT_DLP_SERVER_PORT=50053

ENTRYPOINT ["/app/ytdlp-server"]
