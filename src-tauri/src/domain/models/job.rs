use serde::{Deserialize, Serialize};

/// Represents the status/phase of a sync job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Done => write!(f, "done"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Represents the current download/upload phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobPhase {
    Pending,
    Link,
    Download,
    Upload,
    Done,
}

impl std::fmt::Display for JobPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Link => write!(f, "link"),
            Self::Download => write!(f, "download"),
            Self::Upload => write!(f, "upload"),
            Self::Done => write!(f, "done"),
        }
    }
}

/// A sync job: TorBox → Local → NAS/Drive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: i64,
    pub torrent_id: String,
    pub name: String,
    pub status: JobStatus,
    pub phase: JobPhase,
    pub targets: Vec<String>,
    pub done_targets: Vec<String>,
    pub progress: f64,
    pub speed_bps: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub message: String,
    pub staging_path: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub finished_at: Option<f64>,
}

/// Summary counts of jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCounts {
    pub active: u32,
    pub pending: u32,
    pub done: u32,
    pub failed: u32,
    pub total: u32,
}

/// Enqueue result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueResult {
    pub job_id: i64,
    pub torrent_id: String,
    pub is_new_download: bool,
    pub message: String,
}
