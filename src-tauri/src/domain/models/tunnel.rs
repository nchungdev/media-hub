use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    pub binary: String,
    pub installed: bool,
    pub running: bool,
    pub url: Option<String>,
    pub started_at: Option<String>,
    pub pid: Option<u32>,
    pub error: Option<String>,
}
