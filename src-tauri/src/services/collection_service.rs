use crate::domain::models::collection::{
    CollectionItem, CollectionSummary, CollectionsResponse, DownloadStatus, EpisodeInfo,
    FranchiseGroup, SeasonInfo, SourceCount, SubtitleStatus, SyncStatus,
};
use crate::domain::traits::{ICollectionService, ISettingsService};
use crate::services::job_store::JobStore;
use crate::services::library_aggregator;
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub struct CollectionService {
    settings_service: Arc<dyn ISettingsService>,
    job_store: Option<Arc<JobStore>>,
    cache: RwLock<Option<CollectionsResponse>>,
}

impl CollectionService {
    pub fn new(
        settings_service: Arc<dyn ISettingsService>,
        job_store: Option<Arc<JobStore>>,
    ) -> Self {
        Self {
            settings_service,
            job_store,
            cache: RwLock::new(None),
        }
    }

    fn scan_directory(dir: &Path, media_type: &str, franchise: &str) -> Vec<CollectionItem> {
        let mut items = Vec::new();
        if !dir.exists() {
            return items;
        }

        let re_season = Regex::new(r"(?i)season\s*(\d+)").unwrap();
        let re_tvdb = Regex::new(r"(?i)\{tvdb-(\d+)\}").unwrap();
        let re_year = Regex::new(r"\b(19\d\d|20\d\d)\b").unwrap();
        let re_ep = Regex::new(r"(?i)(?:s\d+)?e(\d+)").unwrap();
        let re_ep_fallback = Regex::new(r"(?i)(?:^|[^\w])(?:ep|episode)\.?\s*(\d+)").unwrap();
        let re_ep_num = Regex::new(r"(?i)(?:-\s*|\s+)(\d{1,4})(?:\s*\[|\s*\.|\s*$)").unwrap();

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
                                    let video_stem =
                                        ep_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                                    let (mut sub_files, mut has_vi, mut has_eng, mut sub_types) =
                                        Self::scan_companion_subtitles(video_stem, &sub_path);

                                    // Quét thêm phụ đề nội bộ (nhúng trong container mkv/mp4)
                                    let internal_subs = Self::scan_internal_subtitles(&ep_path);
                                    for isub in internal_subs {
                                        if isub.lang == "vi" {
                                            has_vi = true;
                                            let st = format!(".vi.{}", isub.format);
                                            if !sub_types.contains(&st) {
                                                sub_types.push(st);
                                            }
                                        }
                                        if isub.lang == "en" {
                                            has_eng = true;
                                        }
                                        sub_files.push(isub);
                                    }

                                    if has_vi {
                                        total_vi_subs += 1;
                                    }

                                    let size_mb = ep_path
                                        .metadata()
                                        .map(|m| (m.len() as f32 / (1024.0 * 1024.0) * 10.0).round() / 10.0)
                                        .unwrap_or(0.0);
                                    let quality = Self::extract_quality(ep_name);

                                    let video_file = crate::domain::models::collection::VideoFileInfo {
                                        name: ep_name.to_string(),
                                        path: ep_path.to_string_lossy().to_string(),
                                        quality,
                                        size_mb,
                                        in_nas: true,
                                        in_gdrive: true,
                                    };

                                    // Trích xuất mã tập (ví dụ S01E35 hoặc fallback)
                                    let e_num_opt = re_ep.captures(ep_name)
                                        .or_else(|| re_ep_fallback.captures(ep_name))
                                        .or_else(|| re_ep_num.captures(ep_name))
                                        .and_then(|cap| cap[1].parse::<u32>().ok());

                                    let ep_key = if let Some(e_num) = e_num_opt {
                                        format!("S{:02}E{:02}", s_num, e_num)
                                    } else {
                                        format!("S{:02}E{:02}", s_num, season.episodes.len() + 1)
                                    };

                                    // Kiểm tra xem tập này đã có trong danh sách chưa (ví dụ đã có bản chất lượng khác)
                                    if let Some(existing_ep) = season.episodes.iter_mut().find(|e| e.key == ep_key) {
                                        existing_ep.video_files.push(video_file);
                                        if has_vi { existing_ep.has_vi_sub = true; }
                                        if has_eng { existing_ep.has_eng_sub = true; }
                                        for st in sub_types {
                                            if !existing_ep.sub_types.contains(&st) {
                                                existing_ep.sub_types.push(st);
                                            }
                                        }
                                        for sf in sub_files {
                                            if !existing_ep.subtitle_files.iter().any(|s| s.path == sf.path) {
                                                existing_ep.subtitle_files.push(sf);
                                            }
                                        }
                                    } else {
                                        let ep_num_label = if let Some(e_num) = e_num_opt {
                                            e_num.to_string()
                                        } else {
                                            (season.episodes.len() + 1).to_string()
                                        };

                                        season.episodes.push(EpisodeInfo {
                                            key: ep_key,
                                            num: ep_num_label,
                                            name: ep_name.to_string(),
                                            video: true,
                                            in_nas: true,
                                            in_gdrive: true,
                                            has_vi_sub: has_vi,
                                            sub_types,
                                            has_eng_sub: has_eng,
                                            subtitle_files: sub_files,
                                            video_files: vec![video_file],
                                        });
                                    }
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
                franchise: franchise.to_string(),
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
                    label: match percent {
                        100 => "🎉 100% Vietsub".to_string(),
                        p if p > 0 => format!("⏳ {}% Vietsub", p),
                        _ => "⚪ Chưa Dịch".to_string(),
                    },
                    color: match percent {
                        100 => "text-emerald-400".to_string(),
                        p if p > 0 => "text-amber-400".to_string(),
                        _ => "text-zinc-400".to_string(),
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

    fn scan_companion_subtitles(
        video_stem: &str,
        dir_path: &Path,
    ) -> (Vec<crate::domain::models::collection::SubtitleFileInfo>, bool, bool, Vec<String>) {
        let mut sub_files = Vec::new();
        let mut has_vi = false;
        let mut has_eng = false;
        let mut sub_types = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(dir_path) {
            for de in dir_entries.flatten() {
                let p = de.path();
                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if fname.starts_with(video_stem) && (fname.ends_with(".ass") || fname.ends_with(".srt") || fname.ends_with(".vtt")) {
                    let is_vi = fname.contains(".vi.") || fname.ends_with(".vi.ass") || fname.ends_with(".vi.srt") || fname.ends_with(".vi.vtt");
                    let is_en = fname.contains(".en.") || fname.contains(".eng.") || fname.ends_with(".en.ass") || fname.ends_with(".en.srt");
                    let lang = if is_vi { "vi" } else if is_en { "en" } else { "other" };
                    let format = if fname.ends_with(".ass") { "ass" } else if fname.ends_with(".srt") { "srt" } else { "vtt" };
                    let size_kb = p.metadata().map(|m| (m.len() as f32 / 1024.0).round()).unwrap_or(0.0);

                    if is_vi {
                        has_vi = true;
                        let st = format!(".vi.{}", format);
                        if !sub_types.contains(&st) { sub_types.push(st); }
                    }
                    if is_en { has_eng = true; }

                    sub_files.push(crate::domain::models::collection::SubtitleFileInfo {
                        name: fname.to_string(), path: p.to_string_lossy().to_string(),
                        lang: lang.to_string(), format: format.to_string(), size_kb,
                        is_internal: false,
                        track_id: None,
                    });
                }
            }
        }
        (sub_files, has_vi, has_eng, sub_types)
    }

    pub fn extract_quality(name: &str) -> String {
        let re_bracket = Regex::new(r"(?i)\[([^\]]*(?:1080p|720p|2160p|4k|bdrip|web-?dl|bluray|remux)[^\]]*)\]").unwrap();
        if let Some(cap) = re_bracket.captures(name) {
            return cap[1].trim().to_string();
        }
        let re_res = Regex::new(r"(?i)\b(2160p|1080p|720p|480p|4k)\b").unwrap();
        if let Some(cap) = re_res.captures(name) {
            return cap[1].to_uppercase();
        }
        "1080p".to_string()
    }

    fn scan_internal_subtitles(video_path: &Path) -> Vec<crate::domain::models::collection::SubtitleFileInfo> {
        let mut list = Vec::new();
        let ffprobe_bin = if std::path::Path::new("/opt/homebrew/bin/ffprobe").exists() {
            "/opt/homebrew/bin/ffprobe"
        } else if std::path::Path::new("/usr/local/bin/ffprobe").exists() {
            "/usr/local/bin/ffprobe"
        } else {
            "ffprobe"
        };

        let output = std::process::Command::new(ffprobe_bin)
            .args(["-v", "quiet", "-print_format", "json", "-show_streams", "-select_streams", "s"])
            .arg(video_path)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    if let Some(streams) = val.get("streams").and_then(|s| s.as_array()) {
                        for st in streams {
                            let idx = st.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
                            let codec = st.get("codec_name").and_then(|c| c.as_str()).unwrap_or("subrip");
                            let format = if codec.contains("ass") {
                                "ass"
                            } else if codec.contains("subrip") || codec.contains("srt") {
                                "srt"
                            } else if codec.contains("vtt") {
                                "vtt"
                            } else {
                                codec
                            };

                            let tags = st.get("tags");
                            let raw_lang = tags.and_then(|t| t.get("language")).and_then(|l| l.as_str()).unwrap_or("und");
                            let title = tags.and_then(|t| t.get("title")).and_then(|t| t.as_str());

                            let lang = if raw_lang.starts_with("vi") || raw_lang == "vie" {
                                "vi"
                            } else if raw_lang.starts_with("en") || raw_lang == "eng" {
                                "en"
                            } else if raw_lang.starts_with("ja") || raw_lang == "jpn" {
                                "ja"
                            } else {
                                "other"
                            };

                            let name = if let Some(t) = title {
                                format!("Track #{}: {} ({})", idx, t, format.to_uppercase())
                            } else {
                                format!("Track #{}: {} ({})", idx, raw_lang.to_uppercase(), format.to_uppercase())
                            };

                            list.push(crate::domain::models::collection::SubtitleFileInfo {
                                name,
                                path: format!("{}#track:{}", video_path.to_string_lossy(), idx),
                                lang: lang.to_string(),
                                format: format.to_string(),
                                size_kb: 0.0,
                                is_internal: true,
                                track_id: Some(idx),
                            });
                        }
                    }
                }
            }
        }
        list
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
        let franchise_root = PathBuf::from(&settings.media_hub_home).join("_franchise");

        // 1. Quét thư mục Draft local (.media-hub/_franchise)
        let mut local_items = Vec::new();
        if let Ok(entries) = fs::read_dir(&franchise_root) {
            for entry in entries.flatten() {
                let franchise_path = entry.path();
                if !franchise_path.is_dir() {
                    continue;
                }
                let fname = entry.file_name();
                let fname = fname.to_string_lossy();
                if fname.starts_with('.') || fname.starts_with('_') {
                    continue;
                }
                let franchise_name = fname.to_string();
                local_items.extend(Self::scan_directory(
                    &franchise_path.join("TV Shows"),
                    "series",
                    &franchise_name,
                ));
                local_items.extend(Self::scan_directory(
                    &franchise_path.join("Movies"),
                    "movie",
                    &franchise_name,
                ));
            }
        }

        // 2. Tích hợp thư viện hợp nhất 3 nguồn (Draft / JellyPlex NAS / Google Drive) từ JobStore
        let mut all_items = local_items;
        let mut counts_detail: HashMap<String, SourceCount> = HashMap::new();

        if let Some(ref js) = self.job_store {
            let unified_lib = library_aggregator::aggregate(js);
            counts_detail = unified_lib.counts_detail;

            let re_year = Regex::new(r"\b(19\d\d|20\d\d)\b").unwrap();
            let re_tvdb = Regex::new(r"(?i)\{tvdb-(\d+)\}").unwrap();

            let mut local_matched: HashSet<usize> = HashSet::new();

            for f_group in unified_lib.franchises {
                for u_item in f_group.items {
                    // Thử khớp với local item
                    let matched_idx = all_items.iter().enumerate().position(|(idx, l_item)| {
                        if local_matched.contains(&idx) {
                            return false;
                        }
                        // So theo tvdb_id nếu có
                        if let (Some(ref l_tid), true) = (&l_item.tvdb_id, u_item.media_key.starts_with("tvdb-")) {
                            if u_item.media_key.trim_start_matches("tvdb-") == l_tid.as_str() {
                                return true;
                            }
                        }
                        // So theo clean title
                        let l_title = l_item.title.trim().to_lowercase();
                        let u_title = u_item.title.trim().to_lowercase();
                        if !l_title.is_empty() && l_title == u_title {
                            return true;
                        }
                        // So theo folder name
                        let l_folder = l_item.folder.trim().to_lowercase();
                        for f in u_item.folders.values() {
                            if !f.is_empty() && f.trim().to_lowercase() == l_folder {
                                return true;
                            }
                        }
                        false
                    });

                    if let Some(idx) = matched_idx {
                        local_matched.insert(idx);
                        all_items[idx].sync.in_nas = u_item.in_nas;
                        all_items[idx].sync.in_gdrive = u_item.in_drive;
                        all_items[idx].sync.in_local = true;
                        if u_item.in_nas && u_item.in_drive {
                            all_items[idx].sync.state = "synced_both".to_string();
                            all_items[idx].sync.label = "🟢 Cả JellyPlex & Drive".to_string();
                            all_items[idx].sync.color = "text-emerald-400".to_string();
                        } else if u_item.in_nas {
                            all_items[idx].sync.state = "only_nas".to_string();
                            all_items[idx].sync.label = "🟡 Chỉ JellyPlex".to_string();
                            all_items[idx].sync.color = "text-amber-400".to_string();
                        } else if u_item.in_drive {
                            all_items[idx].sync.state = "only_gdrive".to_string();
                            all_items[idx].sync.label = "🔵 Chỉ Drive".to_string();
                            all_items[idx].sync.color = "text-blue-400".to_string();
                        } else {
                            all_items[idx].sync.state = "unsynced".to_string();
                            all_items[idx].sync.label = "📝 Chỉ Draft".to_string();
                            all_items[idx].sync.color = "text-zinc-400".to_string();
                        }
                        if all_items[idx].franchise.is_empty() || all_items[idx].franchise == crate::services::library_aggregator::UNCLASSIFIED {
                            if !u_item.franchise.is_empty() {
                                all_items[idx].franchise = u_item.franchise.clone();
                            } else if !f_group.name.is_empty() {
                                all_items[idx].franchise = f_group.name.clone();
                            }
                        }
                    } else {
                        // Mục này chỉ có trên JellyPlex (NAS) hoặc Google Drive, chưa có bản Draft local
                        let folder_name = u_item
                            .folders
                            .get("jellyfin")
                            .or_else(|| u_item.folders.get("plex"))
                            .or_else(|| u_item.folders.get("gdrive"))
                            .cloned()
                            .unwrap_or_else(|| u_item.title.clone());

                        let tvdb_id = if u_item.media_key.starts_with("tvdb-") {
                            Some(u_item.media_key.trim_start_matches("tvdb-").to_string())
                        } else {
                            re_tvdb
                                .captures(&folder_name)
                                .and_then(|c| c.get(1))
                                .map(|m| m.as_str().to_string())
                        };

                        let year = re_year
                            .captures(&folder_name)
                            .or_else(|| re_year.captures(&u_item.title))
                            .and_then(|c| c.get(1))
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_default();

                        let poster_url = if let Some(ref tid) = tvdb_id {
                            format!("/api/poster?tvdb={}", tid)
                        } else {
                            format!("/api/poster?title={}", urlencoding::encode(&u_item.title))
                        };

                        let (sync_state, sync_label, sync_color) = if u_item.in_nas && u_item.in_drive {
                            ("synced_both", "🟢 Cả JellyPlex & Drive", "text-emerald-400")
                        } else if u_item.in_nas {
                            ("only_nas", "🟡 Chỉ JellyPlex", "text-amber-400")
                        } else if u_item.in_drive {
                            ("only_gdrive", "🔵 Chỉ Drive", "text-blue-400")
                        } else {
                            ("unsynced", "📝 Chỉ Draft", "text-zinc-400")
                        };

                        all_items.push(CollectionItem {
                            id: format!("col-{}", u_item.media_key),
                            franchise: if !u_item.franchise.is_empty() {
                                u_item.franchise.clone()
                            } else if !f_group.name.is_empty() {
                                f_group.name.clone()
                            } else {
                                crate::services::library_aggregator::UNCLASSIFIED.to_string()
                            },
                            folder: folder_name,
                            tvdb_id,
                            title: u_item.title.clone(),
                            vn_title: String::new(),
                            year,
                            media_type: u_item.media_type.clone(),
                            poster: poster_url,
                            total_episodes: 1,
                            download: DownloadStatus {
                                state: "cloud".to_string(),
                                label: "☁️ Đám Mây".to_string(),
                                color: "text-zinc-400".to_string(),
                                downloaded: 0,
                                total: 1,
                            },
                            sync: SyncStatus {
                                state: sync_state.to_string(),
                                label: sync_label.to_string(),
                                color: sync_color.to_string(),
                                in_nas: u_item.in_nas,
                                in_gdrive: u_item.in_drive,
                                in_local: false,
                            },
                            subtitle: SubtitleStatus {
                                state: "missing".to_string(),
                                label: "⚪ Chưa Dịch".to_string(),
                                color: "text-zinc-400".to_string(),
                                completed: 0,
                                total: 1,
                                percent: 0,
                            },
                            has_glossary: false,
                            has_progress: false,
                            local_path: None,
                            seasons: Vec::new(),
                        });
                    }
                }
            }
        }

        // Nếu counts_detail chưa có mục nào, tự tính fallback từ all_items
        if counts_detail.is_empty() {
            let mut draft_count = SourceCount::default();
            for it in &all_items {
                if it.download.state == "complete" {
                    if it.media_type == "movie" {
                        draft_count.movies += 1;
                    } else {
                        draft_count.series += 1;
                    }
                    draft_count.total += 1;
                }
            }
            counts_detail.insert("draft".to_string(), draft_count);
        }

        // Sắp xếp: Ưu tiên các mục Draft local lên đầu, tiếp đến các mục remote theo thứ tự chữ cái
        all_items.sort_by(|a, b| {
            let a_local = a.download.state == "complete";
            let b_local = b.download.state == "complete";
            b_local.cmp(&a_local).then(a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });

        let total_items = all_items.len();
        let total_series = all_items.iter().filter(|i| i.media_type == "series").count();
        let total_movies = all_items.iter().filter(|i| i.media_type == "movie").count();
        let downloaded_full = all_items.iter().filter(|i| i.download.state == "complete").count();
        let synced_both = all_items.iter().filter(|i| i.sync.state == "synced_both").count();
        let sub_complete = all_items.iter().filter(|i| i.subtitle.state == "complete").count();

        // Gom nhóm theo franchise
        let mut order: Vec<String> = Vec::new();
        let mut grouped: HashMap<String, FranchiseGroup> = HashMap::new();
        for it in &all_items {
            let g = grouped
                .entry(it.franchise.clone())
                .or_insert_with(|| {
                    order.push(it.franchise.clone());
                    FranchiseGroup {
                        name: it.franchise.clone(),
                        total_items: 0,
                        total_series: 0,
                        total_movies: 0,
                        item_ids: Vec::new(),
                    }
                });
            g.total_items += 1;
            if it.media_type == "series" {
                g.total_series += 1;
            } else {
                g.total_movies += 1;
            }
            g.item_ids.push(it.id.clone());
        }
        order.sort();
        let franchises: Vec<FranchiseGroup> =
            order.into_iter().filter_map(|n| grouped.remove(&n)).collect();

        let resp = CollectionsResponse {
            collections: all_items,
            franchises,
            summary: CollectionSummary {
                total_items,
                total_series,
                total_movies,
                downloaded_full,
                synced_both,
                sub_complete,
            },
            counts_detail,
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
        };

        if let Ok(mut guard) = self.cache.write() {
            *guard = Some(resp.clone());
        }

        resp
    }
}


