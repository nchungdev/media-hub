use serde::{Deserialize, Serialize};

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
pub struct VideoFileInfo {
    pub name: String,
    pub path: String,
    pub quality: String,
    pub size_mb: f32,
    #[serde(default)]
    pub in_nas: bool,
    #[serde(default)]
    pub in_gdrive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleFileInfo {
    pub name: String,
    pub path: String,
    pub lang: String,
    pub format: String,
    pub size_kb: f32,
    #[serde(default)]
    pub is_internal: bool,
    #[serde(default)]
    pub track_id: Option<u32>,
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
    #[serde(default)]
    pub subtitle_files: Vec<SubtitleFileInfo>,
    #[serde(default)]
    pub video_files: Vec<VideoFileInfo>,
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
    /// Ten franchise chua title nay (ten thu muc trong _franchise/).
    /// App hien thi gom nhom theo truong nay, con NAS/Drive van giu
    /// cau truc phang kieu Plex/Jellyfin khi dong bo.
    pub franchise: String,
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
pub struct FranchiseGroup {
    pub name: String,
    pub total_items: usize,
    pub total_series: usize,
    pub total_movies: usize,
    /// Danh sach id cua cac title thuoc franchise nay.
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceCount {
    pub movies: usize,
    pub series: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionItem>,
    /// Gom nhom san theo franchise de UI khong phai tu tinh lai.
    pub franchises: Vec<FranchiseGroup>,
    pub summary: CollectionSummary,
    #[serde(default)]
    pub counts_detail: std::collections::HashMap<String, SourceCount>,
    pub timestamp: f64,
}
