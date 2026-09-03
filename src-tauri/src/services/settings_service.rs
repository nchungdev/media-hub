use crate::domain::models::settings::AppSettings;
use crate::domain::traits::ISettingsService;
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

pub struct SettingsService {
    config_path: PathBuf,
    cache: RwLock<Option<AppSettings>>,
}

impl SettingsService {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let config_path = home.join(".media-hub").join("settings.json");
        Self {
            config_path,
            cache: RwLock::new(None),
        }
    }
}

impl Default for SettingsService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ISettingsService for SettingsService {
    fn load(&self) -> AppSettings {
        if let Ok(guard) = self.cache.read() {
            if let Some(ref cached) = *guard {
                return cached.clone();
            }
        }

        let settings = if self.config_path.exists() {
            match fs::read_to_string(&self.config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppSettings::default(),
            }
        } else {
            AppSettings::default()
        };

        if let Ok(mut guard) = self.cache.write() {
            *guard = Some(settings.clone());
        }

        settings
    }

    fn save(&self, settings: &AppSettings) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, content).map_err(|e| e.to_string())?;

        if let Ok(mut guard) = self.cache.write() {
            *guard = Some(settings.clone());
        }

        Ok(())
    }
}
