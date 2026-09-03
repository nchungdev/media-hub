use crate::domain::traits::ISettingsService;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;

pub struct StreamingService {
    settings_service: Arc<dyn ISettingsService>,
}

impl StreamingService {
    pub fn new(settings_service: Arc<dyn ISettingsService>) -> Self {
        Self { settings_service }
    }

    pub fn find_video_file(&self, path_str: &str) -> Option<(PathBuf, u64)> {
        let path = PathBuf::from(path_str);
        if path.is_file() {
            if let Ok(meta) = path.metadata() {
                return Some((path, meta.len()));
            }
        }

        let settings = self.settings_service.load();
        let ws = PathBuf::from(&settings.workspace_dir);
        let joined = ws.join(path_str.trim_start_matches('/'));
        if joined.is_file() {
            if let Ok(meta) = joined.metadata() {
                return Some((joined, meta.len()));
            }
        }

        None
    }

    pub async fn open_file(&self, path: &Path) -> std::io::Result<File> {
        File::open(path).await
    }
}
