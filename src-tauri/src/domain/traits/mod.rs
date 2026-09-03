// Port interfaces for Dependency Injection in Clean Architecture
use crate::domain::models::{
    collection::CollectionsResponse, quota::QuotaData, settings::AppSettings, tunnel::TunnelStatus,
};
use async_trait::async_trait;

#[async_trait]
pub trait ISettingsService: Send + Sync {
    fn load(&self) -> AppSettings;
    fn save(&self, settings: &AppSettings) -> Result<(), String>;
}

#[async_trait]
pub trait IQuotaService: Send + Sync {
    fn get_status(&self) -> QuotaData;
    fn increment(&self) -> QuotaData;
}

#[async_trait]
pub trait ICollectionService: Send + Sync {
    fn get_collections(&self, refresh: bool) -> CollectionsResponse;
}

#[async_trait]
pub trait ITunnelService: Send + Sync {
    fn get_status(&self) -> TunnelStatus;
    fn start(&self, port: u16, force_new: bool) -> Result<TunnelStatus, String>;
    fn stop(&self) -> Result<TunnelStatus, String>;
}

#[async_trait]
pub trait ISubtitleService: Send + Sync {
    fn srt_to_webvtt(&self, srt: &str) -> String;
    fn ass_to_webvtt(&self, ass: &str) -> String;
}

#[async_trait]
pub trait IArtworkService: Send + Sync {
    async fn resolve_poster(
        &self,
        title: Option<&str>,
        tvdb: Option<&str>,
        tmdb: Option<&str>,
    ) -> Option<Vec<u8>>;
}
