use crate::domain::traits::ISettingsService;
use crate::services::gdrive_service::GDriveService;
use crate::services::nas_service::NasService;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartProposal {
    pub action: String,
    pub label: String,
    pub desc: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowComparison {
    pub folder: String,
    pub tvdb_id: String,
    pub title: Option<String>,
    pub vn: Option<String>,
    pub qual: Option<String>,
    pub poster: String,
    pub in_gdrive: bool,
    pub in_nas: bool,
    pub in_local: bool,
    pub proposals: Vec<SmartProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCheckSummary {
    pub total_shows: usize,
    pub synced_both: usize,
    pub only_gdrive: usize,
    pub only_nas: usize,
    pub need_sub: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCheckResponse {
    pub success: bool,
    pub summary: CrossCheckSummary,
    pub shows: Vec<ShowComparison>,
}

pub struct LibraryService {
    settings: Arc<dyn ISettingsService>,
    gdrive: Arc<GDriveService>,
    nas: Arc<NasService>,
}

impl LibraryService {
    pub fn new(
        settings: Arc<dyn ISettingsService>,
        gdrive: Arc<GDriveService>,
        nas: Arc<NasService>,
    ) -> Self {
        Self {
            settings,
            gdrive,
            nas,
        }
    }

    pub fn cross_check(&self) -> CrossCheckResponse {
        let gdrive_shows_list = self.gdrive.list_tv_shows(false);
        let gdrive_set: HashSet<String> = gdrive_shows_list.into_iter().collect();

        let nas_folders_list = self.nas.list_nas_folders();
        let nas_set: HashSet<String> = nas_folders_list.into_iter().collect();

        let cfg = self.settings.load();
        let staging_dir = &cfg.staging_dir;
        let mut local_set = HashSet::new();
        if let Ok(entries) = std::fs::read_dir(staging_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    if let Ok(name) = entry.file_name().into_string() {
                        local_set.insert(name);
                    }
                }
            }
        }

        let tvdb_re = Regex::new(r"\{tvdb-(\d+)\}").unwrap();
        let all_folders: HashSet<String> = gdrive_set.union(&nas_set).cloned().collect();
        let mut sorted_folders: Vec<String> = all_folders.into_iter().collect();
        sorted_folders.sort();

        let mut comparisons = Vec::new();
        for folder in sorted_folders {
            let in_gdrive = gdrive_set.contains(&folder);
            let in_nas = nas_set.contains(&folder);
            let in_local = local_set.contains(&folder);

            let tvdb_id = tvdb_re
                .captures(&folder)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let title_clean = folder.split('{').next().unwrap_or(&folder).trim().to_string();

            let mut proposals = Vec::new();
            if in_gdrive && !in_nas {
                proposals.push(SmartProposal {
                    action: "sync_to_nas".to_string(),
                    label: "☁️ ➔ 🖥️ Đồng bộ sang NAS".to_string(),
                    desc: "Phim đã có trên Google Drive, đẩy sang NAS Storage qua SSH rclone".to_string(),
                    color: "amber".to_string(),
                });
            } else if in_nas && !in_gdrive {
                proposals.push(SmartProposal {
                    action: "sync_to_drive".to_string(),
                    label: "🖥️ ➔ ☁️ Sao lưu lên Drive".to_string(),
                    desc: "Phim đã có trên NAS, sao lưu lên Google Drive Plex".to_string(),
                    color: "emerald".to_string(),
                });
            }

            if in_gdrive && in_nas {
                proposals.push(SmartProposal {
                    action: "perfect".to_string(),
                    label: "✓ Đã Đồng Bộ Hoàn Hảo".to_string(),
                    desc: "Đã có đầy đủ trên Google Drive & NAS kèm phụ đề Vietsub".to_string(),
                    color: "blue".to_string(),
                });
            }

            let poster_url = if !tvdb_id.is_empty() {
                format!("/api/poster?tvdb={}", tvdb_id)
            } else {
                format!("/api/poster?title={}", urlencoding::encode(&folder))
            };

            comparisons.push(ShowComparison {
                folder,
                tvdb_id,
                title: Some(title_clean.clone()),
                vn: Some(title_clean),
                qual: Some("1080p / 480p".to_string()),
                poster: poster_url,
                in_gdrive,
                in_nas,
                in_local,
                proposals,
            });
        }

        let summary = CrossCheckSummary {
            total_shows: comparisons.len(),
            synced_both: comparisons.iter().filter(|c| c.in_gdrive && c.in_nas).count(),
            only_gdrive: comparisons.iter().filter(|c| c.in_gdrive && !c.in_nas).count(),
            only_nas: comparisons.iter().filter(|c| !c.in_gdrive && c.in_nas).count(),
            need_sub: comparisons.iter().filter(|c| c.proposals.iter().any(|p| p.action == "translate_vietsub")).count(),
        };

        CrossCheckResponse {
            success: true,
            summary,
            shows: comparisons,
        }
    }
}
