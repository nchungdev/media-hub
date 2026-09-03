use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct RefreshQuery {
    #[serde(default)]
    pub refresh: Option<String>,
}

pub async fn handle_gdrive_shows(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RefreshQuery>,
) -> Json<Value> {
    let refresh = query
        .refresh
        .map(|r| r == "1" || r == "true")
        .unwrap_or(false);

    let shows_raw = state.gdrive.list_tv_shows(refresh);
    let shows: Vec<Value> = shows_raw
        .into_iter()
        .map(|s| json!({ "name": s, "folder": s }))
        .collect();

    Json(json!({ "shows": shows }))
}

#[derive(Deserialize)]
pub struct SeasonFilesQuery {
    #[serde(default)]
    pub show: String,
    #[serde(default)]
    pub season: String,
}

pub async fn handle_gdrive_season_files(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SeasonFilesQuery>,
) -> Json<Value> {
    if query.show.is_empty() || query.season.is_empty() {
        return Json(json!({ "files": [] }));
    }

    let files = state.gdrive.get_season_files(&query.show, &query.season);
    Json(json!({ "files": files }))
}

#[derive(Deserialize)]
pub struct GdriveCheckRequest {
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_root")]
    pub root: String,
}

fn default_remote() -> String {
    "gdrive".to_string()
}
fn default_root() -> String {
    "Phim/TV Shows".to_string()
}

pub async fn handle_gdrive_check(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GdriveCheckRequest>,
) -> Json<Value> {
    match state.gdrive.check_connection(&payload.remote, &payload.root) {
        Ok(dirs) => Json(json!({
            "success": true,
            "message": format!("Kết nối tới {}:{} thành công! (Tìm thấy {} thư mục TV Shows)", payload.remote, payload.root, dirs.len()),
            "dirs": dirs.into_iter().take(10).collect::<Vec<_>>()
        })),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}
