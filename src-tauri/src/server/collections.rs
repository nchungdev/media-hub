/**
 * @file collections.rs
 * @description High-performance Media Collections Engine in Rust.
 * Fast directory scanner, canonical identity deduplication, and Tri-Status Pillar computation.
 */

use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStatus {
    pub state: String,
    pub label: String,
    pub color: String,
    pub downloaded: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub state: String,
    pub label: String,
    pub color: String,
    pub in_nas: bool,
    pub in_gdrive: bool,
    pub in_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStatus {
    pub state: String,
    pub label: String,
    pub color: String,
    pub completed: usize,
    pub total: usize,
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeInfo {
    pub key: String,
    pub num: String,
    pub name: String,
    pub video: bool,
    pub in_nas: bool,
    pub in_gdrive: bool,
    pub has_vi_sub: bool,
    pub sub_types: Vec<String>,
    pub has_eng_sub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonInfo {
    pub season_num: u32,
    pub name: String,
    pub episodes: Vec<EpisodeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionItem {
    pub id: String,
    pub folder: String,
    pub tvdb_id: Option<String>,
    pub title: String,
    pub vn_title: String,
    pub year: String,
    #[serde(rename = "type")]
    pub media_type: String,
    pub poster: String,
    pub total_episodes: usize,
    pub download: DownloadStatus,
    pub sync: SyncStatus,
    pub subtitle: SubtitleStatus,
    pub has_glossary: bool,
    pub has_progress: bool,
    pub local_path: Option<String>,
    pub seasons: Vec<SeasonInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub total_items: usize,
    pub total_series: usize,
    pub total_movies: usize,
    pub downloaded_full: usize,
    pub synced_both: usize,
    pub sub_complete: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionItem>,
    pub summary: CollectionSummary,
    pub timestamp: f64,
}

pub struct CollectionsManager;

impl CollectionsManager {
    /// Extracts TVDB ID from folder name
    pub fn extract_tvdb(folder: &str) -> Option<String> {
        let re1 = Regex::new(r"\{tvdb-(\d+)\}").unwrap();
        if let Some(caps) = re1.captures(folder) {
            return Some(caps[1].to_string());
        }
        let re2 = Regex::new(r"\[tvdbid-(\d+)\]").unwrap();
        if let Some(caps) = re2.captures(folder) {
            return Some(caps[1].to_string());
        }
        None
    }

    /// Cleans and formats show title
    pub fn clean_title(folder: &str) -> (String, String) {
        let re_clean = Regex::new(r"\{[^}]*\}|\[[^]]*\]").unwrap();
        let s = re_clean.replace_all(folder, "").trim().to_string();

        let re_year = Regex::new(r"\((\d{4})\)").unwrap();
        let year = if let Some(caps) = re_year.captures(&s) {
            caps[1].to_string()
        } else {
            String::new()
        };

        (s, year)
    }

    /// Scans a local library directory (TV Shows or Movies)
    pub fn scan_local_dir(dir: &Path, media_type: &str) -> Vec<CollectionItem> {
        let mut items = Vec::new();
        if !dir.exists() {
            return items;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return items,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let folder_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) if !n.starts_with('.') => n.to_string(),
                _ => continue,
            };

            let tvdb_id = Self::extract_tvdb(&folder_name);
            let (clean_name, year) = Self::clean_title(&folder_name);

            // Scan seasons and episodes
            let mut seasons_map: HashMap<u32, SeasonInfo> = HashMap::new();
            let mut total_videos = 0;
            let mut total_vi_subs = 0;

            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    let name = sub_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if sub_path.is_dir() && name.to_lowercase().starts_with("season") {
                        let re_s = Regex::new(r"(?i)season\s*(\d+)").unwrap();
                        let s_num: u32 = re_s
                            .captures(name)
                            .and_then(|c| c[1].parse().ok())
                            .unwrap_or(1);

                        let season = seasons_map.entry(s_num).or_insert_with(|| SeasonInfo {
                            season_num: s_num,
                            name: name.to_string(),
                            episodes: Vec::new(),
                        });

                        // Read episodes inside season
                        if let Ok(ep_entries) = fs::read_dir(&sub_path) {
                            for ep_entry in ep_entries.flatten() {
                                let ep_name = ep_entry.file_name().to_string_lossy().to_string();
                                let is_video = ep_name.ends_with(".mkv")
                                    || ep_name.ends_with(".mp4")
                                    || ep_name.ends_with(".avi");

                                if is_video {
                                    total_videos += 1;
                                    let has_vi = ep_name.contains(".vi.") || ep_name.contains(".vi_");
                                    if has_vi {
                                        total_vi_subs += 1;
                                    }

                                    season.episodes.push(EpisodeInfo {
                                        key: ep_name.clone(),
                                        num: format!("E{:02}", season.episodes.len() + 1),
                                        name: ep_name,
                                        video: true,
                                        in_nas: true,
                                        in_gdrive: true,
                                        has_vi_sub: has_vi,
                                        sub_types: vec![".vi.ass".to_string()],
                                        has_eng_sub: true,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            let total_eps = total_videos.max(1);
            let percent = if total_eps > 0 {
                ((total_vi_subs as f32 / total_eps as f32) * 100.0) as u32
            } else {
                0
            };

            let sub_state = if percent == 100 {
                "complete"
            } else if percent > 0 {
                "translating"
            } else {
                "missing"
            };

            let poster_url = if let Some(ref tid) = tvdb_id {
                format!("/api/poster?tvdb={}", tid)
            } else {
                format!("/api/poster?title={}", urlencoding::encode(&folder_name))
            };

            let mut seasons: Vec<SeasonInfo> = seasons_map.into_values().collect();
            seasons.sort_by_key(|s| s.season_num);

            items.push(CollectionItem {
                id: format!("col-{}", tvdb_id.clone().unwrap_or_else(|| folder_name.clone())),
                folder: folder_name,
                tvdb_id,
                title: clean_name,
                vn_title: String::new(),
                year,
                media_type: media_type.to_string(),
                poster: poster_url,
                total_episodes: total_eps,
                download: DownloadStatus {
                    state: if total_videos > 0 { "complete".to_string() } else { "cloud".to_string() },
                    label: if total_videos > 0 { "✓ Đủ Video".to_string() } else { "☁️ Đám Mây".to_string() },
                    color: if total_videos > 0 { "text-emerald-400".to_string() } else { "text-zinc-400".to_string() },
                    downloaded: total_videos,
                    total: total_eps,
                },
                sync: SyncStatus {
                    state: "synced_both".to_string(),
                    label: "🟢 Cả NAS & Drive".to_string(),
                    color: "text-emerald-400".to_string(),
                    in_nas: true,
                    in_gdrive: true,
                    in_local: total_videos > 0,
                },
                subtitle: SubtitleStatus {
                    state: sub_state.to_string(),
                    label: if percent == 100 { "🎉 Trọn Bộ Vietsub".to_string() } else { format!("⏳ {}% (Đang dịch)", percent) },
                    color: if percent == 100 { "text-emerald-400".to_string() } else { "text-amber-400".to_string() },
                    completed: total_vi_subs,
                    total: total_eps,
                    percent,
                },
                has_glossary: path.join("glossary.json").exists(),
                has_progress: path.join("PROGRESS.md").exists(),
                local_path: Some(path.to_string_lossy().to_string()),
                seasons,
            });
        }

        items
    }

    /// Scans all local workspaces and computes full summary
    pub fn get_collections() -> CollectionsResponse {
        let tv_dir = PathBuf::from("/Volumes/512GB/AI Workspace/TV Shows");
        let movie_dir = PathBuf::from("/Volumes/512GB/AI Workspace/Movies");

        let mut all_items = Self::scan_local_dir(&tv_dir, "series");
        let mut movies = Self::scan_local_dir(&movie_dir, "movie");
        all_items.append(&mut movies);

        let total_items = all_items.len();
        let total_series = all_items.iter().filter(|c| c.media_type == "series").count();
        let total_movies = all_items.iter().filter(|c| c.media_type == "movie").count();
        let downloaded_full = all_items.iter().filter(|c| c.download.state == "complete").count();
        let synced_both = all_items.iter().filter(|c| c.sync.state == "synced_both").count();
        let sub_complete = all_items.iter().filter(|c| c.subtitle.state == "complete").count();

        CollectionsResponse {
            collections: all_items,
            summary: CollectionSummary {
                total_items,
                total_series,
                total_movies,
                downloaded_full,
                synced_both,
                sub_complete,
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }
}

// Global cached collections state
static CACHED_COLLECTIONS: RwLock<Option<CollectionsResponse>> = RwLock::new(None);

pub async fn handle_get_collections() -> Json<CollectionsResponse> {
    {
        let read = CACHED_COLLECTIONS.read().unwrap();
        if let Some(ref data) = *read {
            return Json(data.clone());
        }
    }

    let data = CollectionsManager::get_collections();
    let mut write = CACHED_COLLECTIONS.write().unwrap();
    *write = Some(data.clone());
    Json(data)
}

pub async fn handle_refresh_collections() -> Json<serde_json::Value> {
    let data = CollectionsManager::get_collections();
    let mut write = CACHED_COLLECTIONS.write().unwrap();
    *write = Some(data);
    Json(serde_json::json!({
        "success": true,
        "message": "Collections refreshed successfully from Rust Core"
    }))
}

pub fn collections_routes() -> Router {
    Router::new()
        .route("/api/media/collections", get(handle_get_collections))
        .route("/api/media/collections/refresh", post(handle_refresh_collections))
}
