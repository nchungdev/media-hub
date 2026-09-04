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
        let ws_config = PathBuf::from("/Volumes/512GB/AI Workspace/.media-hub/_app/config.json");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_config = home.join(".media-hub").join("settings.json");

        let config_path = if ws_config.exists() {
            ws_config
        } else if home_config.exists() {
            home_config
        } else {
            home.join(".media-hub").join("settings.json")
        };

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

        let mut settings = if self.config_path.exists() {
            match fs::read_to_string(&self.config_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => AppSettings::default(),
            }
        } else {
            AppSettings::default()
        };

        // Ensure staging_dir is outside the .app bundle so it is never deleted when updating/reinstalling the app.
        // Khong con .staging rieng o root nua -- moi noi tai ve deu nam trong _franchise/,
        // ke ca truong hop chua xac dinh duoc franchise (_Unsorted).
        let old_root_staging = "/Volumes/512GB/AI Workspace/.media-hub/.staging";
        if settings.staging_dir.is_empty()
            || settings.staging_dir.contains("Contents/Resources")
            || settings.staging_dir == old_root_staging
        {
            settings.staging_dir =
                "/Volumes/512GB/AI Workspace/.media-hub/_franchise/_Unsorted/.staging".to_string();
        }

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
