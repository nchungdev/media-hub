use crate::domain::traits::IArtworkService;
use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct PosterParams {
    pub title: Option<String>,
    pub tvdb: Option<String>,
    pub tmdb: Option<String>,
}

pub async fn handle_poster(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PosterParams>,
) -> Response {
    if let Some(bytes) = state
        .artwork
        .resolve_poster(
            params.title.as_deref(),
            params.tvdb.as_deref(),
            params.tmdb.as_deref(),
        )
        .await
    {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            bytes,
        )
            .into_response()
    } else {
        static PLACEHOLDER_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 300" width="200" height="300"><rect width="200" height="300" fill="#18181b"/><text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle" font-size="40" fill="#71717a">🎬</text></svg>"##;
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml"),
                (header::CACHE_CONTROL, "public, max-age=86400"),
            ],
            PLACEHOLDER_SVG.as_bytes().to_vec(),
        )
            .into_response()
    }
}
