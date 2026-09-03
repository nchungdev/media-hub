use crate::domain::models::collection::{
    CollectionItem, CollectionSummary, CollectionsResponse, DownloadStatus, EpisodeInfo,
    SeasonInfo, SubtitleStatus, SyncStatus,
};
use crate::domain::traits::{ICollectionService, ISettingsService};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub struct CollectionService {
    settings_service: Arc<dyn ISettingsService>,
    cache: RwLock<Option<CollectionsResponse>>,
}

impl CollectionService {
    pub fn new(settings_service: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings_service,
            cache: RwLock::new(None),
        }
    }

    fn scan_directory(dir: &Path, media_type: &str) -> Vec<CollectionItem> {
        let mut items = Vec::new();
        if !dir.exists() {
            return items;
        }

        let re_season = Regex::new(r"(?i)season\s*(\d+)").unwrap();
        let re_tvdb = Regex::new(r"(?i)\{tvdb-(\d+)\}").unwrap();
        let re_year = Regex::new(r"\b(19\d\d|20\d\d)\b").unwrap();

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

            let tvdb_id = re_tvdb
                .captures(&folder_name)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());

            let clean_name = re_tvdb.replace_all(&folder_name, "").trim().to_string();
            let year = re_year
                .captures(&folder_name)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let mut seasons_map: HashMap<u32, SeasonInfo> = HashMap::new();
            let mut total_videos = 0;
            let mut total_vi_subs = 0;

            if let Ok(sub_entries) = fs::read_dir(&path) {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    let name = sub_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if sub_path.is_dir() && name.to_lowercase().starts_with("season") {
                        let s_num: u32 = re_season
                            .captures(name)
                            .and_then(|c| c[1].parse().ok())
                            .unwrap_or(1);

                        let season = seasons_map.entry(s_num).or_insert_with(|| SeasonInfo {
                            season_num: s_num,
                            name: name.to_string(),
                            episodes: Vec::new(),
                        });

                        if let Ok(ep_entries) = fs::read_dir(&sub_path) {
                            for ep_entry in ep_entries.flatten() {
                                let ep_path = ep_entry.path();
                                let ep_name =
                                    ep_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                let is_video = ep_name.ends_with(".mkv")
                                    || ep_name.ends_with(".mp4")
                                    || ep_name.ends_with(".m4v");
                                if is_video {
                                    total_videos += 1;
                                    let has_vi = ep_path.with_extension("vi.ass").exists()
                                        || ep_path.with_extension("vi.srt").exists();
                                    if has_vi {
                                        total_vi_subs += 1;
                                    }
                                    season.episodes.push(EpisodeInfo {
                                        key: ep_name.to_string(),
                                        num: format!("E{:02}", season.episodes.len() + 1),
                                        name: ep_name.to_string(),
                                        video: true,
                                        in_nas: true,
                                        in_gdrive: true,
                                        has_vi_sub: has_vi,
                                        sub_types: if has_vi {
                                            vec!["vi".to_string()]
                                        } else {
                                            vec![]
                                        },
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
                id: format!(
                    "col-{}",
                    tvdb_id.clone().unwrap_or_else(|| folder_name.clone())
                ),
                folder: folder_name,
                tvdb_id,
                title: clean_name,
                vn_title: String::new(),
                year,
                media_type: media_type.to_string(),
                poster: poster_url,
                total_episodes: total_eps,
                download: DownloadStatus {
                    state: if total_videos > 0 {
                        "complete".to_string()
                    } else {
                        "cloud".to_string()
                    },
                    label: if total_videos > 0 {
                        "✓ Đủ Video".to_string()
                    } else {
                        "☁️ Đám Mây".to_string()
                    },
                    color: if total_videos > 0 {
                        "text-emerald-400".to_string()
                    } else {
                        "text-zinc-400".to_string()
                    },
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
                    label: if percent == 100 {
                        "🎉 100% Vietsub".to_string()
                    } else if percent > 0 {
                        format!("⏳ {}% Vietsub", percent)
                    } else {
                        "⚪ Chưa Dịch".to_string()
                    },
                    color: if percent == 100 {
                        "text-emerald-400".to_string()
                    } else if percent > 0 {
                        "text-amber-400".to_string()
                    } else {
                        "text-zinc-400".to_string()
                    },
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
}

#[async_trait]
impl ICollectionService for CollectionService {
    fn get_collections(&self, refresh: bool) -> CollectionsResponse {
        if !refresh {
            if let Ok(guard) = self.cache.read() {
                if let Some(ref cached) = *guard {
                    return cached.clone();
                }
            }
        }

        let settings = self.settings_service.load();
        let ws = PathBuf::from(&settings.workspace_dir);
        let tv_dir = ws.join(&settings.tv_dirname);
        let movie_dir = ws.join(&settings.movies_dirname);

        let mut all_items = Vec::new();
        all_items.extend(Self::scan_directory(&tv_dir, "series"));
        all_items.extend(Self::scan_directory(&movie_dir, "movie"));

        let total_items = all_items.len();
        let total_series = all_items
            .iter()
            .filter(|i| i.media_type == "series")
            .count();
        let total_movies = all_items.iter().filter(|i| i.media_type == "movie").count();
        let downloaded_full = all_items
            .iter()
            .filter(|i| i.download.state == "complete")
            .count();
        let synced_both = all_items
            .iter()
            .filter(|i| i.sync.state == "synced_both")
            .count();
        let sub_complete = all_items
            .iter()
            .filter(|i| i.subtitle.state == "complete")
            .count();

        let resp = CollectionsResponse {
            collections: all_items,
            summary: CollectionSummary {
                total_items,
                total_series,
                total_movies,
                downloaded_full,
                synced_both,
                sub_complete,
            },
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        };

        if let Ok(mut guard) = self.cache.write() {
            *guard = Some(resp.clone());
        }

        resp
    }
}
