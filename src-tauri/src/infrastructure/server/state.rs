use crate::domain::traits::{
    ICollectionService, IQuotaService, ISettingsService, ISubtitleService, ITunnelService,
};
use crate::services::{
    agent_service::AgentService, artwork_service::ArtworkService,
    collection_service::CollectionService, dashboard_service::DashboardService,
    gdrive_service::GDriveService, health_service::HealthService, job_store::JobStore,
    library_service::LibraryService, nas_service::NasService, quota_service::QuotaService,
    settings_service::SettingsService, streaming_service::StreamingService,
    subtitle_service::SubtitleService, tmdb_service::TmdbService, torbox_service::TorboxService,
    tunnel_service::TunnelService,
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
    pub job_store: Arc<JobStore>,
    pub dashboard: Arc<DashboardService>,
    pub gdrive: Arc<GDriveService>,
    pub nas: Arc<NasService>,
    pub health: Arc<HealthService>,
    pub library: Arc<LibraryService>,
    pub tmdb: Arc<TmdbService>,
    pub agent: Arc<AgentService>,
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

        let job_store = Arc::new(JobStore::new(None).expect("Failed to init JobStore"));
        let dashboard = Arc::new(DashboardService::new(settings.clone(), job_store.clone()));
        let gdrive = Arc::new(GDriveService::new(settings.clone()));
        let nas = Arc::new(NasService::new(settings.clone()));
        let health = Arc::new(HealthService::new(settings.clone()));
        let library = Arc::new(LibraryService::new(
            settings.clone(),
            gdrive.clone(),
            nas.clone(),
        ));
        let tmdb = Arc::new(TmdbService::new(settings.clone()));
        let agent = Arc::new(AgentService::new(settings.clone()));

        Self {
            settings,
            quota,
            collections,
            artwork,
            subtitles,
            streaming,
            torbox,
            tunnel,
            job_store,
            dashboard,
            gdrive,
            nas,
            health,
            library,
            tmdb,
            agent,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

