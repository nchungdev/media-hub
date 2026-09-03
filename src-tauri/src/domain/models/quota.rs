use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaData {
    pub day_str: String,
    pub daily_count: u32,
    pub daily_limit: u32,
    pub week_str: String,
    pub weekly_count: u32,
    pub weekly_limit: u32,
    pub is_locked: bool,
    pub remaining_today: u32,
    pub remaining_this_week: u32,
    pub updated_at: f64,
}

impl Default for QuotaData {
    fn default() -> Self {
        Self {
            day_str: String::new(),
            daily_count: 0,
            daily_limit: 30,
            week_str: String::new(),
            weekly_count: 0,
            weekly_limit: 150,
            is_locked: false,
            remaining_today: 30,
            remaining_this_week: 150,
            updated_at: 0.0,
        }
    }
}
