/**
 * @file quota_guard.rs
 * @description Translation Quota Guard in Rust for Gemini Flash safe quotas.
 */

use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

const DAILY_LIMIT: u32 = 30;
const WEEKLY_LIMIT: u32 = 150;

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
        let (day, week) = Self::get_date_keys();
        Self {
            day_str: day,
            daily_count: 0,
            daily_limit: DAILY_LIMIT,
            week_str: week,
            weekly_count: 0,
            weekly_limit: WEEKLY_LIMIT,
            is_locked: false,
            remaining_today: DAILY_LIMIT,
            remaining_this_week: WEEKLY_LIMIT,
            updated_at: Self::now_ts(),
        }
    }
}

impl QuotaData {
    fn now_ts() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    fn get_date_keys() -> (String, String) {
        let ts = Self::now_ts() as i64;
        let days = ts / 86400;
        let day_str = format!("day-{}", days);
        let week_str = format!("week-{}", days / 7);
        (day_str, week_str)
    }

    pub fn refresh_cycles(&mut self) {
        let (cur_day, cur_week) = Self::get_date_keys();
        if self.day_str != cur_day {
            self.day_str = cur_day;
            self.daily_count = 0;
        }
        if self.week_str != cur_week {
            self.week_str = cur_week;
            self.weekly_count = 0;
        }
        self.remaining_today = self.daily_limit.saturating_sub(self.daily_count);
        self.remaining_this_week = self.weekly_limit.saturating_sub(self.weekly_count);
        self.is_locked = self.daily_count >= self.daily_limit || self.weekly_count >= self.weekly_limit;
        self.updated_at = Self::now_ts();
    }
}

pub struct QuotaGuardManager;

impl QuotaGuardManager {
    pub fn get_file_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".media-hub");
        let _ = fs::create_dir_all(&dir);
        dir.join("quota_guard.json")
    }

    pub fn load() -> QuotaData {
        let path = Self::get_file_path();
        let mut data = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            QuotaData::default()
        };

        data.refresh_cycles();
        data
    }

    pub fn save(data: &QuotaData) {
        let path = Self::get_file_path();
        if let Ok(c) = serde_json::to_string_pretty(data) {
            let _ = fs::write(path, c);
        }
    }
}

static CACHED_QUOTA: RwLock<Option<QuotaData>> = RwLock::new(None);

pub async fn handle_get_quota() -> Json<QuotaData> {
    let mut data = QuotaGuardManager::load();
    data.refresh_cycles();
    let mut write = CACHED_QUOTA.write().unwrap();
    *write = Some(data.clone());
    Json(data)
}

pub async fn handle_increment_quota() -> Json<QuotaData> {
    let mut data = QuotaGuardManager::load();
    data.refresh_cycles();
    data.daily_count += 1;
    data.weekly_count += 1;
    data.refresh_cycles();

    QuotaGuardManager::save(&data);

    let mut write = CACHED_QUOTA.write().unwrap();
    *write = Some(data.clone());
    Json(data)
}

pub fn quota_routes() -> Router {
    Router::new()
        .route("/api/quota/status", get(handle_get_quota))
        .route("/api/quota/increment", post(handle_increment_quota))
}
