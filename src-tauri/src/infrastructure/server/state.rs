use crate::domain::traits::{
    ICollectionService, IQuotaService, ISettingsService, ISubtitleService, ITunnelService,
};
use crate::services::{
    watcher_service,
    agent_service::AgentService, artwork_service::ArtworkService,
    collection_service::CollectionService, dashboard_service::DashboardService,
    gdrive_service::GDriveService, health_service::HealthService, job_store::JobStore,
    library_service::LibraryService, nas_service::NasService, quota_service::QuotaService,
    settings_service::SettingsService, streaming_service::StreamingService,
    subtitle_service::SubtitleService, tmdb_service::TmdbService, torbox_service::TorboxService,
    tunnel_service::TunnelService,
};
use std::path::PathBuf;
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

        // Mot noi duy nhat cho toan bo trang thai: media_hub_home tu config.json,
        // hoac $HOME/.media-hub neu chua cau hinh.
        let cfg = settings.load();
        let home: PathBuf = if !cfg.media_hub_home.is_empty() {
            PathBuf::from(cfg.media_hub_home.clone())
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".media-hub")
        };
        let app_state_dir = home.join("_app");
        let _ = std::fs::create_dir_all(&app_state_dir);

        let quota = Arc::new(QuotaService::new(app_state_dir.clone()));
        let artwork = Arc::new(ArtworkService::new(app_state_dir.clone()));
        let subtitles = Arc::new(SubtitleService::new());
        let tunnel = Arc::new(TunnelService::new(app_state_dir.clone()));
        let collections: Arc<dyn ICollectionService> = Arc::new(CollectionService::new(settings.clone()));
        let streaming = Arc::new(StreamingService::new(settings.clone()));
        let torbox = Arc::new(TorboxService::new(settings.clone()));

        let job_store = Arc::new(
            JobStore::new(Some(app_state_dir.join("media_hub.db"))).expect("Failed to init JobStore"),
        );

        // Watcher nen: theo doi .media-hub/_franchise, tu dong lam moi + luu
        // collections_cache vao DB moi khi co thay doi file (them/xoa/doi ten).
        watcher_service::start(home.join("_franchise"), collections.clone(), job_store.clone());
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

