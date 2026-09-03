/**
 * @file artwork.rs
 * @description Fast Multi-Tiered Poster & Artwork Resolver in Rust.
 * Tiers:
 * 1. Disk Cache: ~/.media-hub/.cache/posters/
 * 2. Local File: poster.jpg in local series folder
 * 3. AniList GraphQL API / Kitsu API fallback
 */

use axum::{
    extract::Query,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct PosterParams {
    pub title: Option<String>,
    pub tvdb: Option<String>,
    pub tmdb: Option<String>,
}

pub struct ArtworkResolver;

impl ArtworkResolver {
    /// Gets cache directory path ~/.media-hub/.cache/posters/
    pub fn get_cache_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home)
            .join(".media-hub")
            .join(".cache")
            .join("posters");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Computes safe cache filename hash from key
    pub fn compute_cache_key(key: &str) -> String {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:016x}.jpg", hasher.finish())
    }

    /// Tries fetching poster from AniList GraphQL API
    pub async fn fetch_anilist_poster(title: &str) -> Option<Vec<u8>> {
        let clean_title = title
            .split('{')
            .next()?
            .split('(')
            .next()?
            .trim();

        if clean_title.is_empty() {
            return None;
        }

        let query = r#"
            query ($search: String) {
                Media (search: $search, type: ANIME) {
                    coverImage {
                        extraLarge
                        large
                    }
                }
            }
        "#;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .ok()?;

        let body = serde_json::json!({
            "query": query,
            "variables": { "search": clean_title }
        });

        let resp = client
            .post("https://graphql.anilist.co")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        let img_url = json["data"]["Media"]["coverImage"]["extraLarge"]
            .as_str()
            .or_else(|| json["data"]["Media"]["coverImage"]["large"].as_str())?;

        // Download image bytes
        let img_resp = client.get(img_url).send().await.ok()?;
        if img_resp.status().is_success() {
            img_resp.bytes().await.ok().map(|b| b.to_vec())
        } else {
            None
        }
    }

    /// Resolves poster bytes through multi-tiered strategy
    pub async fn resolve_poster(params: &PosterParams) -> Option<Vec<u8>> {
        let key = params
            .tvdb
            .as_deref()
            .or(params.tmdb.as_deref())
            .or(params.title.as_deref())?;

        let cache_dir = Self::get_cache_dir();
        let cache_file = cache_dir.join(Self::compute_cache_key(key));

        // Tier 1: Check Disk Cache
        if cache_file.exists() {
            if let Ok(bytes) = fs::read(&cache_file) {
                if !bytes.is_empty() {
                    return Some(bytes);
                }
            }
        }

        // Tier 2: Search local workspace folders for poster.jpg
        let workspace_dirs = [
            "/Volumes/512GB/AI Workspace/TV Shows",
            "/Volumes/512GB/AI Workspace/Movies",
            "/Volumes/512GB/AI Workspace",
        ];

        for base in &workspace_dirs {
            let p = Path::new(base).join(key);
            if p.exists() {
                for poster_name in &["poster.jpg", "cover.jpg", "folder.jpg", "poster.png"] {
                    let poster_path = p.join(poster_name);
                    if poster_path.exists() {
                        if let Ok(bytes) = fs::read(&poster_path) {
                            let _ = fs::write(&cache_file, &bytes);
                            return Some(bytes);
                        }
                    }
                }
            }
        }

        // Tier 3: Fetch from AniList GraphQL
        if let Some(title) = params.title.as_deref() {
            if let Some(bytes) = Self::fetch_anilist_poster(title).await {
                let _ = fs::write(&cache_file, &bytes);
                return Some(bytes);
            }
        }

        None
    }
}

/// Axum Handler for GET /api/poster
pub async fn handle_poster(Query(params): Query<PosterParams>) -> Response {
    if let Some(bytes) = ArtworkResolver::resolve_poster(&params).await {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
        headers.insert(
            header::CACHE_CONTROL,
            "public, max-age=604800, immutable".parse().unwrap(),
        );
        (StatusCode::OK, headers, bytes).into_response()
    } else {
        // Return 1x1 transparent SVG placeholder or 404
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="450" viewBox="0 0 300 450"><rect width="100%" height="100%" fill="#18181b"/><text x="50%" y="50%" fill="#71717a" dominant-baseline="middle" text-anchor="middle" font-family="sans-serif" font-size="14">No Poster Available</text></svg>"##;
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "image/svg+xml".parse().unwrap());
        (StatusCode::OK, headers, svg).into_response()
    }
}
