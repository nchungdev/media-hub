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

    /// Lay link tai truc tiep (HTTPS) cho mot file da duoc TorBox cache san.
    /// Day la buoc noi giua "TorBox da cache tren server ho" va "may minh tai ve":
    /// TorBox tra ve mot URL co chu ky, het han sau mot thoi gian ngan, va tu do
    /// tro di no chi la mot link HTTP binh thuong -- aria2c tai duoc nhu moi link khac.
    pub async fn request_download_link(
        &self,
        torrent_id: u64,
        file_id: Option<u64>,
    ) -> Result<String, String> {
        let token = self.get_token();
        if token.is_empty() {
            return Err("TorBox API Token chưa được cấu hình".to_string());
        }

        // Endpoint nay nhan token qua query param chu khong phai bearer header.
        let mut url = format!(
            "https://api.torbox.app/v1/api/torrents/requestdl?token={}&torrent_id={}",
            token, torrent_id
        );
        if let Some(fid) = file_id {
            url.push_str(&format!("&file_id={}", fid));
        } else {
            // Khong chi dinh file cu the -> lay ca torrent duoi dang mot file zip.
            url.push_str("&zip_link=true");
        }

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body: Value = resp.json().await.map_err(|e| e.to_string())?;

        if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
            let detail = body
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("TorBox tu choi cap link tai");
            return Err(detail.to_string());
        }

        body.get("data")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "TorBox tra ve du lieu khong co link".to_string())
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
