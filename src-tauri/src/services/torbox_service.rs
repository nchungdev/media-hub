use crate::domain::traits::ISettingsService;
use serde_json::Value;
use std::sync::Arc;

pub struct TorboxService {
    settings_service: Arc<dyn ISettingsService>,
    client: reqwest::Client,
}

impl TorboxService {
    pub fn new(settings_service: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings_service,
            client: reqwest::Client::new(),
        }
    }

    fn get_token(&self) -> String {
        let s = self.settings_service.load();
        if !s.torbox_token.is_empty() {
            s.torbox_token
        } else {
            std::env::var("TORBOX_API_KEY").unwrap_or_default()
        }
    }

    pub async fn list_torrents(&self, bypass_cache: bool) -> Result<Value, String> {
        let token = self.get_token();
        if token.is_empty() {
            return Err("TorBox API Token chưa được cấu hình".to_string());
        }

        let url = format!(
            "https://api.torbox.app/v1/api/torrents/mylist?bypass_cache={}",
            bypass_cache
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<Value>().await.map_err(|e| e.to_string())
    }

    pub async fn add_torrent(&self, magnet: &str) -> Result<Value, String> {
        let token = self.get_token();
        if token.is_empty() {
            return Err("TorBox API Token chưa được cấu hình".to_string());
        }

        let mut form = std::collections::HashMap::new();
        form.insert("magnet", magnet);

        let resp = self
            .client
            .post("https://api.torbox.app/v1/api/torrents/createtorrent")
            .bearer_auth(token)
            .form(&form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<Value>().await.map_err(|e| e.to_string())
    }

    pub async fn delete_torrent(&self, torrent_id: u64) -> Result<Value, String> {
        let token = self.get_token();
        if token.is_empty() {
            return Err("TorBox API Token chưa được cấu hình".to_string());
        }

        let resp = self
            .client
            .post("https://api.torbox.app/v1/api/torrents/controltorrent")
            .bearer_auth(token)
            .json(&serde_json::json!({
                "torrent_id": torrent_id,
                "operation": "delete"
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<Value>().await.map_err(|e| e.to_string())
    }
}
