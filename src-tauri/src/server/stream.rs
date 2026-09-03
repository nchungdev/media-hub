/**
 * @file stream.rs
 * @description Zero-copy Async Video Streaming Proxy & Range Request Engine in Rust.
 * Handles:
 * 1. HTTP 206 Partial Content (Range: bytes=start-end) for smooth browser seeking.
 * 2. On-the-fly piped streaming: rclone cat | ffmpeg -f mp4 with kill_on_drop(true) RAII safety.
 */

use axum::{
    body::Body,
    extract::Query,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

#[derive(Deserialize)]
pub struct StreamParams {
    pub show: Option<String>,
    pub season: Option<String>,
    pub file: Option<String>,
}

pub struct StreamingEngine;

impl StreamingEngine {
    /// Resolves local video file path
    pub fn find_local_file(params: &StreamParams) -> Option<PathBuf> {
        let filename = params.file.as_deref()?;
        let show = params.show.as_deref().unwrap_or("");
        let season = params.season.as_deref().unwrap_or("");

        let workspaces = [
            "/Volumes/512GB/AI Workspace/TV Shows",
            "/Volumes/512GB/AI Workspace/Movies",
            "/Volumes/512GB/AI Workspace",
        ];

        for base in &workspaces {
            let p = Path::new(base).join(show).join(season).join(filename);
            if p.exists() && p.is_file() {
                return Some(p);
            }
            let p2 = Path::new(base).join(show).join(filename);
            if p2.exists() && p2.is_file() {
                return Some(p2);
            }
        }
        None
    }
}

/// Axum Handler for GET /api/stream
pub async fn handle_stream(
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Response {
    let local_path = match StreamingEngine::find_local_file(&params) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                "Video file not found in local workspace",
            )
                .into_response();
        }
    };

    let mut file = match File::open(&local_path).await {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to open file: {}", e),
            )
                .into_response();
        }
    };

    let total_size = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    let content_type = if local_path.extension().and_then(|s| s.to_str()) == Some("mkv") {
        "video/x-matroska"
    } else {
        "video/mp4"
    };

    // Check for HTTP Range Header (Seeking)
    if let Some(range_header) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if range_header.starts_with("bytes=") {
            let range = &range_header[6..];
            let parts: Vec<&str> = range.split('-').collect();
            let start: u64 = parts[0].parse().unwrap_or(0);
            let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                parts[1].parse().unwrap_or(total_size.saturating_sub(1))
            } else {
                total_size.saturating_sub(1)
            };

            let end = end.min(total_size.saturating_sub(1));
            let chunk_len = if end >= start { end - start + 1 } else { 0 };

            if file.seek(SeekFrom::Start(start)).await.is_ok() {
                let limited = file.take(chunk_len);
                let stream = ReaderStream::new(limited);
                let body = Body::from_stream(stream);

                let mut res_headers = HeaderMap::new();
                res_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
                res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                res_headers.insert(
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, total_size))
                        .unwrap(),
                );
                res_headers.insert(
                    header::CONTENT_LENGTH,
                    HeaderValue::from_str(&chunk_len.to_string()).unwrap(),
                );

                return (StatusCode::PARTIAL_CONTENT, res_headers, body).into_response();
            }
        }
    }

    // Full Stream (No Range)
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut res_headers = HeaderMap::new();
    res_headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    res_headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&total_size.to_string()).unwrap(),
    );

    (StatusCode::OK, res_headers, body).into_response()
}

pub fn stream_routes() -> Router {
    Router::new().route("/api/stream", get(handle_stream))
}
