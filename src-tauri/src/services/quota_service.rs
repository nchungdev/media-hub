use crate::domain::models::quota::QuotaData;
use crate::domain::traits::IQuotaService;
use async_trait::async_trait;
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

pub struct QuotaService {
    state_file: PathBuf,
    data: RwLock<QuotaData>,
}

impl QuotaService {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let state_file = home.join(".media-hub").join("translation_quota.json");
        let initial_data = Self::load_file(&state_file);
        Self {
            state_file,
            data: RwLock::new(initial_data),
        }
    }

    fn load_file(path: &PathBuf) -> QuotaData {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(mut data) = serde_json::from_str::<QuotaData>(&content) {
                    Self::sync_time_windows(&mut data);
                    return data;
                }
            }
        }
        let mut d = QuotaData::default();
        Self::sync_time_windows(&mut d);
        d
    }

    fn sync_time_windows(data: &mut QuotaData) {
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        let this_week = now.format("%Y-W%U").to_string();

        if data.day_str != today {
            data.day_str = today;
            data.daily_count = 0;
        }

        if data.week_str != this_week {
            data.week_str = this_week;
            data.weekly_count = 0;
        }

        data.remaining_today = data.daily_limit.saturating_sub(data.daily_count);
        data.remaining_this_week = data.weekly_limit.saturating_sub(data.weekly_count);
        data.is_locked = data.remaining_today == 0 || data.remaining_this_week == 0;
        data.updated_at = now.timestamp() as f64;
    }

    fn persist(&self, data: &QuotaData) {
        if let Some(parent) = self.state_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(data) {
            let _ = fs::write(&self.state_file, json);
        }
    }
}

impl Default for QuotaService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IQuotaService for QuotaService {
    fn get_status(&self) -> QuotaData {
        let mut guard = self.data.write().unwrap();
        Self::sync_time_windows(&mut guard);
        self.persist(&guard);
        guard.clone()
    }

    fn increment(&self) -> QuotaData {
        let mut guard = self.data.write().unwrap();
        Self::sync_time_windows(&mut guard);
        if !guard.is_locked {
            guard.daily_count += 1;
            guard.weekly_count += 1;
            Self::sync_time_windows(&mut guard);
            self.persist(&guard);
        }
        guard.clone()
    }
}
