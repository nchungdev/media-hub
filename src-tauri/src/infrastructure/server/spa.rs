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
        PathBuf::from(format!("frontend/{}", filename)),
        PathBuf::from(format!("../frontend/{}", filename)),
        PathBuf::from(format!("templates/{}", filename)),
        PathBuf::from(format!("../templates/{}", filename)),
        PathBuf::from(format!("/Volumes/512GB/Studio Projects/media-hub/frontend/{}", filename)),
        PathBuf::from(format!("/Volumes/512GB/Studio Projects/media-hub/templates/{}", filename)),
    ];
    for c in &candidates {
        if c.exists() && c.is_file() {
            return Some(c.clone());
        }
    }
    None
}

async fn render_template_with_includes(path: &PathBuf) -> Result<String, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut rendered = content;
    let re = regex::Regex::new(r#"<!--\s*include\s*["']([^"']+)["']\s*-->"#).unwrap();

    let mut iterations = 0;
    while iterations < 5 && re.is_match(&rendered) {
        iterations += 1;
        let mut replacements = Vec::new();
        for cap in re.captures_iter(&rendered) {
            if let (Some(full_match), Some(sub_path)) = (cap.get(0), cap.get(1)) {
                let sub_path_str = sub_path.as_str();
                if let Some(partial_file) = find_template_file(sub_path_str) {
                    if let Ok(partial_content) = tokio::fs::read_to_string(&partial_file).await {
                        replacements.push((full_match.as_str().to_string(), partial_content));
                    }
                }
            }
        }
        if replacements.is_empty() {
            break;
        }
        for (placeholder, replacement) in replacements {
            rendered = rendered.replace(&placeholder, &replacement);
        }
    }
    Ok(rendered)
}

pub async fn handle_spa_index() -> impl IntoResponse {
    if let Some(path) = find_template_file("index.html") {
        match render_template_with_includes(&path).await {
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
        PathBuf::from(format!("frontend/static/{}", clean_path)),
        PathBuf::from(format!("../frontend/static/{}", clean_path)),
        PathBuf::from(format!("static/{}", clean_path)),
        PathBuf::from(format!("../static/{}", clean_path)),
        PathBuf::from(format!("templates/static/{}", clean_path)),
        PathBuf::from(format!("../templates/static/{}", clean_path)),
        PathBuf::from(format!("/Volumes/512GB/Studio Projects/media-hub/frontend/static/{}", clean_path)),
        PathBuf::from(format!("/Volumes/512GB/Studio Projects/media-hub/static/{}", clean_path)),
    ];

    let mut found_path = None;
    for c in &candidates {
        if c.exists() && c.is_file() {
            found_path = Some(c.clone());
            break;
        }
    }

    let path = match found_path {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    let mime_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .as_ref()
        .to_string();

    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let cache_control = if clean_path.ends_with(".js") || clean_path.ends_with(".css") {
                "no-cache, must-revalidate"
            } else {
                "public, max-age=3600"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type)
                .header(header::CACHE_CONTROL, cache_control)
                .body(Body::from(bytes))
                .unwrap_or_else(|_| {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Builder error").into_response()
                })
        }
        Err(_) => (StatusCode::NOT_FOUND, "File read error").into_response(),
    }
}
