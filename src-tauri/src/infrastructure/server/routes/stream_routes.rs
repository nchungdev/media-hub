use crate::infrastructure::server::state::AppState;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::io::SeekFrom;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

#[derive(Debug, Deserialize)]
pub struct StreamParams {
    pub path: Option<String>,
}

pub async fn handle_stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Response {
    let path_str = match params.path {
        Some(ref p) if !p.is_empty() => p,
        _ => return (StatusCode::BAD_REQUEST, "Missing path parameter").into_response(),
    };

    let (file_path, total_size) = match state.streaming.find_video_file(path_str) {
        Some(res) => res,
        None => return (StatusCode::NOT_FOUND, "Video file not found").into_response(),
    };

    let mime_type = if path_str.ends_with(".mp4") {
        "video/mp4"
    } else if path_str.ends_with(".mkv") {
        "video/x-matroska"
    } else {
        "video/webm"
    };

    // Range requests
    if let Some(range_header) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(range) = range_header.strip_prefix("bytes=") {
            let parts: Vec<&str> = range.split('-').collect();
            let start: u64 = parts[0].parse().unwrap_or(0);
            let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                parts[1].parse().unwrap_or(total_size.saturating_sub(1))
            } else {
                total_size.saturating_sub(1)
            };

            let end = end.min(total_size.saturating_sub(1));
            let chunk_len = if end >= start { end - start + 1 } else { 0 };

            if let Ok(mut f) = state.streaming.open_file(&file_path).await {
                if f.seek(SeekFrom::Start(start)).await.is_ok() {
                    let take = f.take(chunk_len);
                    let stream = ReaderStream::new(take);
                    let body = Body::from_stream(stream);

                    return (
                        StatusCode::PARTIAL_CONTENT,
                        [
                            (header::CONTENT_TYPE, mime_type),
                            (
                                header::CONTENT_RANGE,
                                &format!("bytes {}-{}/{}", start, end, total_size),
                            ),
                            (header::CONTENT_LENGTH, &chunk_len.to_string()),
                            (header::ACCEPT_RANGES, "bytes"),
                        ],
                        body,
                    )
                        .into_response();
                }
            }
        }
    }

    // Full Content
    if let Ok(f) = state.streaming.open_file(&file_path).await {
        let stream = ReaderStream::new(f);
        let body = Body::from_stream(stream);

        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime_type),
                (header::CONTENT_LENGTH, &total_size.to_string()),
                (header::ACCEPT_RANGES, "bytes"),
            ],
            body,
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to open video file",
        )
            .into_response()
    }
}
