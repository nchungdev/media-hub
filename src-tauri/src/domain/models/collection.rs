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
pub struct SubtitleFileInfo {
    pub name: String,
    pub path: String,
    pub lang: String,
    pub format: String,
    pub size_kb: f32,
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
