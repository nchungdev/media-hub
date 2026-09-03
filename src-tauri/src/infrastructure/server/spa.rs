use axum::{
    body::Body,
    extract::Path as AxumPath,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::path::PathBuf;


fn find_template_file(filename: &str) -> Option<PathBuf> {
    // Check possible locations
    let candidates = [
        PathBuf::from(format!("templates/{}", filename)),
        PathBuf::from(format!("../templates/{}", filename)),
        PathBuf::from(format!("../../templates/{}", filename)),
        PathBuf::from(format!("/Volumes/512GB/AI Workspace/apps/media-hub/templates/{}", filename)),
    ];
    for c in &candidates {
        if c.exists() && c.is_file() {
            return Some(c.clone());
        }
    }
    None
}

pub async fn handle_spa_index() -> impl IntoResponse {
    if let Some(path) = find_template_file("index.html") {
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Html(content).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read index.html: {}", e),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Html("<h1>Antigravity Media Hub</h1><p>Template index.html not found.</p>"),
        )
            .into_response()
    }
}

pub async fn handle_static(AxumPath(file_path): AxumPath<String>) -> Response {
    let clean_path = file_path.trim_start_matches('/');
    if clean_path.contains("..") {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let candidates = [
        PathBuf::from(format!("static/{}", clean_path)),
        PathBuf::from(format!("../static/{}", clean_path)),
        PathBuf::from(format!("/Volumes/512GB/AI Workspace/apps/media-hub/static/{}", clean_path)),
        PathBuf::from(format!("templates/{}", clean_path)),
        PathBuf::from(format!("../templates/{}", clean_path)),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_file() {
            if let Ok(bytes) = tokio::fs::read(candidate).await {
                let mime = mime_guess::from_path(candidate)
                    .first_or_octet_stream()
                    .as_ref()
                    .to_string();

                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CACHE_CONTROL, "public, max-age=86400")
                    .body(Body::from(bytes))
                    .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response());
            }
        }
    }

    (StatusCode::NOT_FOUND, "Static file not found").into_response()
}
