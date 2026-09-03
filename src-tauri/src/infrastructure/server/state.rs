use crate::domain::traits::{
    ICollectionService, IQuotaService, ISettingsService, ISubtitleService, ITunnelService,
};
use crate::services::{
    artwork_service::ArtworkService, collection_service::CollectionService,
    quota_service::QuotaService, settings_service::SettingsService,
    streaming_service::StreamingService, subtitle_service::SubtitleService,
    torbox_service::TorboxService, tunnel_service::TunnelService,
};
use std::sync::Arc;

pub struct AppState {
    pub settings: Arc<dyn ISettingsService>,
    pub quota: Arc<dyn IQuotaService>,
    pub collections: Arc<dyn ICollectionService>,
    pub artwork: Arc<ArtworkService>,
    pub subtitles: Arc<dyn ISubtitleService>,
    pub streaming: Arc<StreamingService>,
    pub torbox: Arc<TorboxService>,
    pub tunnel: Arc<dyn ITunnelService>,
}

impl AppState {
    pub fn new() -> Self {
        let settings = Arc::new(SettingsService::new());
        let quota = Arc::new(QuotaService::new());
        let artwork = Arc::new(ArtworkService::new());
        let subtitles = Arc::new(SubtitleService::new());
        let tunnel = Arc::new(TunnelService::new());
        let collections = Arc::new(CollectionService::new(settings.clone()));
        let streaming = Arc::new(StreamingService::new(settings.clone()));
        let torbox = Arc::new(TorboxService::new(settings.clone()));

        Self {
            settings,
            quota,
            collections,
            artwork,
            subtitles,
            streaming,
            torbox,
            tunnel,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
