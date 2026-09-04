use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use regex::Regex;

#[derive(Debug, Deserialize)]
pub struct SubtitleParams {
    pub path: Option<String>,
}

pub async fn handle_subtitle(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SubtitleParams>,
) -> Response {
    let path_str = match params.path {
        Some(ref p) if !p.is_empty() => p,
        _ => return (StatusCode::BAD_REQUEST, "Missing path parameter").into_response(),
    };

    let p = PathBuf::from(path_str);
    if !p.exists() {
        return (StatusCode::NOT_FOUND, "Subtitle file not found").into_response();
    }

    let content = match fs::read_to_string(&p) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read subtitle file",
            )
                .into_response()
        }
    };

    let vtt = if path_str.ends_with(".ass") || path_str.ends_with(".ssa") {
        state.subtitles.ass_to_webvtt(&content)
    } else {
        state.subtitles.srt_to_webvtt(&content)
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/vtt; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        vtt,
    )
        .into_response()
}

#[derive(Debug, Serialize, Clone)]
pub struct EpisodeInfo {
    pub key: String,
    pub season_num: u32,
    pub ep_num: u32,
    pub video: bool,
    pub vi_ass: bool,
    pub vi_ass_path: String,
    pub vi_ass_name: String,
    pub vi_srt: bool,
    pub vi_srt_path: String,
    pub vi_srt_name: String,
    pub vi_vtt: bool,
    pub vi_vtt_path: String,
    pub vi_vtt_name: String,
    pub eng_sub: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct SubtitleProject {
    pub name: String,
    pub title: String,
    pub tmdb_id: Option<Value>,
    pub tvdb_id: Option<Value>,
    pub total_episodes: usize,
    pub completed_episodes: usize,
    pub percent: f64,
    pub has_glossary: bool,
    pub has_progress: bool,
    pub path: String,
    pub episodes: Vec<EpisodeInfo>,
}

fn walk_episodes(dir: &Path, episodes: &mut BTreeMap<String, EpisodeInfo>, re_ep: &Regex) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current_dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let fname = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) if !n.starts_with('.') => n.to_string(),
                    _ => continue,
                };

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(caps) = re_ep.captures(&fname) {
                        let s_num: u32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
                        let e_num: u32 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
                        let ep_key = format!("S{:02}E{:02}", s_num, e_num);

                        let ep = episodes.entry(ep_key.clone()).or_insert_with(|| EpisodeInfo {
                            key: ep_key,
                            season_num: s_num,
                            ep_num: e_num,
                            video: false,
                            vi_ass: false,
                            vi_ass_path: String::new(),
                            vi_ass_name: String::new(),
                            vi_srt: false,
                            vi_srt_path: String::new(),
                            vi_srt_name: String::new(),
                            vi_vtt: false,
                            vi_vtt_path: String::new(),
                            vi_vtt_name: String::new(),
                            eng_sub: false,
                        });

                        let lower = fname.to_lowercase();
                        if lower.ends_with(".mkv") || lower.ends_with(".mp4") || lower.ends_with(".m4v") || lower.ends_with(".avi") {
                            ep.video = true;
                        } else if lower.ends_with(".vi.ass") {
                            ep.vi_ass = true;
                            ep.vi_ass_path = path.to_string_lossy().to_string();
                            ep.vi_ass_name = fname;
                        } else if lower.ends_with(".vi.srt") {
                            ep.vi_srt = true;
                            ep.vi_srt_path = path.to_string_lossy().to_string();
                            ep.vi_srt_name = fname;
                        } else if lower.ends_with(".vi.vtt") {
                            ep.vi_vtt = true;
                            ep.vi_vtt_path = path.to_string_lossy().to_string();
                            ep.vi_vtt_name = fname;
                        } else if lower.ends_with(".eng.ass") || lower.ends_with(".eng.srt") || lower.ends_with(".ass") || lower.ends_with(".srt") {
                            ep.eng_sub = true;
                        }
                    }
                }
            }
        }
    }
}

pub async fn handle_subtitle_projects(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let cfg = state.settings.load();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    
    let candidate_homes = [
        PathBuf::from("/Volumes/512GB/AI Workspace/.media-hub"),
        PathBuf::from(&cfg.media_hub_home),
        home.join(".media-hub"),
        PathBuf::from(".media-hub"),
    ];

    let hub_home = candidate_homes.into_iter()
        .find(|p| p.exists() && p.is_dir())
        .unwrap_or_else(|| home.join(".media-hub"));

    let re_ep = Regex::new(r"(?i)S(\d+)E(\d+)").unwrap();
    let mut projects: Vec<SubtitleProject> = Vec::new();

    if hub_home.exists() {
        if let Ok(entries) = fs::read_dir(&hub_home) {
            for entry in entries.flatten() {
                let item_path = entry.path();
                let item_name = entry.file_name().to_string_lossy().to_string();
                if item_name.starts_with('.') || !item_path.is_dir() {
                    continue;
                }

                let tv_dir = item_path.join("TV Shows");
                if tv_dir.exists() && tv_dir.is_dir() {
                    if let Ok(shows) = fs::read_dir(&tv_dir) {
                        for show_entry in shows.flatten() {
                            let sp = show_entry.path();
                            let show_name = show_entry.file_name().to_string_lossy().to_string();
                            if show_name.starts_with('.') || !sp.is_dir() {
                                continue;
                            }

                            let meta_file = sp.join("metadata.json");
                            let prog_file = sp.join("PROGRESS.md");
                            let gloss_file = sp.join("glossary.json");

                            let meta: Value = if meta_file.exists() {
                                fs::read_to_string(&meta_file)
                                    .ok()
                                    .and_then(|c| serde_json::from_str(&c).ok())
                                    .unwrap_or(Value::Null)
                            } else {
                                Value::Null
                            };

                            let title = meta.get("title")
                                .and_then(|t| t.as_str())
                                .unwrap_or(&show_name)
                                .to_string();
                            let tmdb_id = meta.get("tmdb_id").cloned();
                            let tvdb_id = meta.get("tvdb_id").cloned();

                            let mut episodes_map: BTreeMap<String, EpisodeInfo> = BTreeMap::new();
                            walk_episodes(&sp, &mut episodes_map, &re_ep);

                            let total_eps = episodes_map.len().max(
                                meta.get("total_episodes")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as usize)
                                    .unwrap_or(episodes_map.len())
                            );

                            let completed = episodes_map.values()
                                .filter(|ep| ep.vi_ass || ep.vi_srt || ep.vi_vtt)
                                .count();

                            if total_eps > 0 {
                                let percent = if total_eps > 0 {
                                    ((completed as f64 / total_eps as f64) * 1000.0).round() / 10.0
                                } else {
                                    0.0
                                };

                                projects.push(SubtitleProject {
                                    name: show_name,
                                    title,
                                    tmdb_id,
                                    tvdb_id,
                                    total_episodes: total_eps,
                                    completed_episodes: completed,
                                    percent,
                                    has_glossary: gloss_file.exists(),
                                    has_progress: prog_file.exists(),
                                    path: sp.to_string_lossy().to_string(),
                                    episodes: episodes_map.into_values().collect(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    projects.sort_by(|a, b| {
        let a_done = if a.percent >= 100.0 { 1 } else { 0 };
        let b_done = if b.percent >= 100.0 { 1 } else { 0 };
        if a_done != b_done {
            return a_done.cmp(&b_done);
        }
        b.percent.partial_cmp(&a.percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });

    Json(json!({ "projects": projects }))
}
