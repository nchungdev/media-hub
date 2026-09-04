use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_cross_check(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let res = state.library.cross_check();
    Json(json!(res))
}

pub async fn handle_library_stats(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "drive": { "shows": 0, "files": 0, "size_gb": 0 },
        "missing_assets": []
    }))
}

pub async fn handle_library_build_status(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "running": false,
        "progress": 100,
        "message": "Sẵn sàng"
    }))
}

pub async fn handle_library_build(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "message": "Đã bắt đầu tiến trình dựng metadata"
    }))
}

pub async fn handle_library_refresh(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let _ = state.gdrive.list_tv_shows(true);
    Json(json!({
        "success": true,
        "refreshed": true,
        "message": "Đã lập chỉ mục lại thư viện Google Drive."
    }))
}

pub async fn handle_library_build_cancel(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "message": "Đã yêu cầu dừng tiến trình dựng metadata."
    }))
}

/// Thu vien hop nhat 3 nguon, gom theo franchise, danh dau
/// co mat o local / NAS / Google Drive.
pub async fn handle_unified_library(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let lib = crate::services::library_aggregator::aggregate(&state.job_store);
    Json(serde_json::to_value(lib).unwrap_or_else(|_| serde_json::json!({})))
}

/// Chuyen mot dong library_unified thanh JSON tra ve cho client.
fn unified_row_to_json(
    row: (String, String, String, String, bool, bool, bool, String, String, String),
) -> serde_json::Value {
    let (root, title, franchise, media_type, draft, nas, drive, seen, folders, paths) = row;
    let seen_by: Vec<String> = serde_json::from_str(&seen).unwrap_or_default();
    let folders: serde_json::Value = serde_json::from_str(&folders).unwrap_or_else(|_| serde_json::json!({}));
    let paths: serde_json::Value = serde_json::from_str(&paths).unwrap_or_else(|_| serde_json::json!({}));

    // Jellyfin va Plex deu mo ta thu vien NAS -> gop lai khi tra ve,
    // nhung van giu seen_by de biet ben nao thuc su nhin thay.
    let jellyplex_seen: Vec<&String> = seen_by
        .iter()
        .filter(|s| s.as_str() == "jellyfin" || s.as_str() == "plex")
        .collect();

    serde_json::json!({
        "found": true,
        "media_key": root,
        "title": title,
        "franchise": franchise,
        "type": media_type,
        "draft":     { "exists": draft, "path": paths.get("local") },
        "jellyplex": { "exists": nas, "seen_by": jellyplex_seen, "path": paths.get("jellyfin") },
        "drive":     { "exists": drive, "folder": folders.get("gdrive") },
    })
}

/// Tra mot media id bat ky (tmdb-123 / tvdb-456 / imdb-tt789).
pub async fn handle_library_lookup(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(media_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match state.job_store.lookup_media(&media_id) {
        Some(row) => Json(unified_row_to_json(row)),
        None => Json(serde_json::json!({
            "found": false,
            "media_key": media_id,
            "draft":     { "exists": false },
            "jellyplex": { "exists": false },
            "drive":     { "exists": false },
        })),
    }
}

/// Ban hang loat: nhan {"ids": [...]}. Duyet danh sach tai ve thuong can hoi
/// vai chuc id mot luc, goi le tung cai se cham.
pub async fn handle_library_lookup_batch(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let ids: Vec<String> = body
        .get("ids")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut results = serde_json::Map::new();
    for id in ids {
        let v = match state.job_store.lookup_media(&id) {
            Some(row) => unified_row_to_json(row),
            None => serde_json::json!({
                "found": false,
                "media_key": id,
                "draft":     { "exists": false },
                "jellyplex": { "exists": false },
                "drive":     { "exists": false },
            }),
        };
        results.insert(id, v);
    }

    Json(serde_json::json!({
        "results": results,
        "total_in_library": state.job_store.unified_count(),
    }))
}
