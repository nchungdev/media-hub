/**
 * @file torbox.rs
 * @description TorBox Cloud Debrid REST API Client in Rust.
 */

use axum::{
    extract::Query,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TorboxAddPayload {
    pub magnet: Option<String>,
}

#[derive(Deserialize)]
pub struct TorboxCacheQuery {
    pub hash: Option<String>,
}

#[derive(Deserialize)]
pub struct TorboxDeletePayload {
    pub torrent_id: Option<i64>,
}

pub struct TorboxClient;

impl TorboxClient {
    pub fn get_token() -> String {
        let settings = super::settings::SettingsManager::load();
        settings.torbox_token
    }

    pub async fn list_torrents() -> serde_json::Value {
        let token = Self::get_token();
        if token.is_empty() {
            return serde_json::json!({
                "success": false,
                "error": "TorBox token chưa được cấu hình"
            });
        }

        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.torbox.app/v1/api/torrents/mylist?bypass_cache=true")
            .bearer_auth(token)
            .send()
            .await;

        match resp {
            Ok(r) => r.json().await.unwrap_or(serde_json::json!([])),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    pub async fn check_cache(hash: &str) -> serde_json::Value {
        let token = Self::get_token();
        let client = reqwest::Client::new();
        let url = format!("https://api.torbox.app/v1/api/torrents/checkcached?hash={}&format=object", hash);

        let mut req = client.get(&url);
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }

        match req.send().await {
            Ok(r) => r.json().await.unwrap_or(serde_json::json!({ "success": false })),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    pub async fn add_torrent(magnet: &str) -> serde_json::Value {
        let token = Self::get_token();
        if token.is_empty() {
            return serde_json::json!({ "success": false, "error": "Chưa có token" });
        }

        let client = reqwest::Client::new();
        let mut form = std::collections::HashMap::new();
        form.insert("magnet", magnet);

        match client
            .post("https://api.torbox.app/v1/api/torrents/createtorrent")
            .bearer_auth(token)
            .form(&form)
            .send()
            .await
        {
            Ok(r) => r.json().await.unwrap_or(serde_json::json!({ "success": false })),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    pub async fn delete_torrent(id: i64) -> serde_json::Value {
        let token = Self::get_token();
        let client = reqwest::Client::new();
        let body = serde_json::json!({ "torrent_id": id });

        match client
            .delete("https://api.torbox.app/v1/api/torrents/controltorrent")
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r.json().await.unwrap_or(serde_json::json!({ "success": true })),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }
}

pub async fn handle_list_torrents() -> Json<serde_json::Value> {
    let res = TorboxClient::list_torrents().await;
    Json(res)
}

pub async fn handle_add_torrent(Json(payload): Json<TorboxAddPayload>) -> Json<serde_json::Value> {
    let magnet = payload.magnet.unwrap_or_default();
    let res = TorboxClient::add_torrent(&magnet).await;
    Json(res)
}

pub async fn handle_check_cache(Query(q): Query<TorboxCacheQuery>) -> Json<serde_json::Value> {
    let hash = q.hash.unwrap_or_default();
    let res = TorboxClient::check_cache(&hash).await;
    Json(res)
}

pub async fn handle_delete_torrent(Json(payload): Json<TorboxDeletePayload>) -> Json<serde_json::Value> {
    if let Some(id) = payload.torrent_id {
        let res = TorboxClient::delete_torrent(id).await;
        Json(res)
    } else {
        Json(serde_json::json!({ "success": false, "error": "Missing torrent_id" }))
    }
}

pub fn torbox_routes() -> Router {
    Router::new()
        .route("/api/torbox/torrents", get(handle_list_torrents))
        .route("/api/torbox/add", post(handle_add_torrent))
        .route("/api/torbox/cache", get(handle_check_cache))
        .route("/api/torbox/torrent", delete(handle_delete_torrent))
}
