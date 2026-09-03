use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct NasScanRequest {
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_user")]
    pub user: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub path: String,
}

fn default_user() -> String {
    "admin".to_string()
}
fn default_port() -> u16 {
    22
}

pub async fn handle_nas_scan(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NasScanRequest>,
) -> Json<Value> {
    match state.nas.scan_nas(
        &payload.host,
        &payload.user,
        payload.port,
        &payload.key,
        &payload.path,
    ) {
        Ok(libraries) => Json(json!({
            "success": true,
            "libraries": libraries
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}
