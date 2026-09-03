use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SubtitleParams {
    pub path: Option<String>,
}

pub async fn handle_subtitle(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SubtitleParams>,
) -> Response {
    let path_str = match params.path {
        Some(ref p) if !p.is_empty() => p,
        _ => return (StatusCode::BAD_REQUEST, "Missing path parameter").into_response(),
    };

    let p = PathBuf::from(path_str);
    if !p.exists() {
        return (StatusCode::NOT_FOUND, "Subtitle file not found").into_response();
    }

    let content = match fs::read_to_string(&p) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read subtitle file",
            )
                .into_response()
        }
    };

    let vtt = if path_str.ends_with(".ass") || path_str.ends_with(".ssa") {
        state.subtitles.ass_to_webvtt(&content)
    } else {
        state.subtitles.srt_to_webvtt(&content)
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/vtt; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        vtt,
    )
        .into_response()
}
