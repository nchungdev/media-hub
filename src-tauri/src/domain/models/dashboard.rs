use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDiskProbe {
    pub name: String,
    pub path: String,
    pub total_gb: Option<f64>,
    pub used_gb: Option<f64>,
    pub free_gb: Option<f64>,
    pub percent: u32,
    pub measured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub cpu_load: String,
    pub ram_total_gb: String,
    pub ram_used_gb: String,
    pub ram_pct: u32,
    pub local_disk: LocalDiskProbe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudStorageInfo {
    pub id: String,
    pub icon: String,
    pub name: String,
    pub path: String,
    pub connected: bool,
    pub used_str: String,
    pub avail_str: String,
    pub total_str: String,
    pub percent: u32,
    pub badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCard {
    pub job_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_path: Option<String>,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_ep: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ep: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentMediaItem {
    pub id: String,
    pub title: String,
    pub vn: String,
    pub year: String,
    pub qual: String,
    pub episodes: String,
    pub sub: String,
    pub dest: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub success: bool,
    pub measured_at: f64,
    pub health: SystemHealth,
    pub clouds: Vec<CloudStorageInfo>,
    pub active_downloads: Vec<TransferCard>,
    pub active_uploads: Vec<TransferCard>,
    pub recent_media: Vec<RecentMediaItem>,
    pub job_counts: serde_json::Value,
}
