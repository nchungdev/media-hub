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
        StatusCode::NOT_FOUND.into_response()
    }
}
