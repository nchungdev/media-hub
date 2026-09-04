use crate::domain::traits::ICollectionService;
use crate::services::job_store::JobStore;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Lang nghe thay doi trong .media-hub/_franchise (them/xoa/doi ten file, thu muc)
/// va tu dong lam moi cache collections + ghi lai vao SQLite (job_store) de FE
/// khong can bam refresh thu cong sau moi lan them phim/tap moi.
pub fn start(franchise_root: PathBuf, collections: Arc<dyn ICollectionService>, job_store: Arc<JobStore>) {
    if !franchise_root.exists() {
        let _ = std::fs::create_dir_all(&franchise_root);
    }

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = match new_debouncer(Duration::from_secs(3), tx) {
            Ok(d) => d,
            Err(e) => {
                log::error!("[watcher] khong khoi tao duoc: {}", e);
                return;
            }
        };

        if let Err(e) = debouncer
            .watcher()
            .watch(&franchise_root, notify::RecursiveMode::Recursive)
        {
            log::error!("[watcher] khong watch duoc {:?}: {}", franchise_root, e);
            return;
        }

        log::info!("[watcher] dang lang nghe thay doi tai {:?}", franchise_root);

        // Quet lan dau ngay khi khoi dong, dam bao DB co du lieu tu dau.
        refresh_and_persist(&collections, &job_store);

        for result in rx {
            match result {
                Ok(events) => {
                    let real_change = events
                        .iter()
                        .any(|e| matches!(e.kind, DebouncedEventKind::Any));
                    if real_change {
                        log::info!(
                            "[watcher] phat hien {} thay doi -> lam moi collections",
                            events.len()
                        );
                        refresh_and_persist(&collections, &job_store);
                    }
                }
                Err(e) => log::warn!("[watcher] loi su kien: {:?}", e),
            }
        }
    });
}

fn refresh_and_persist(collections: &Arc<dyn ICollectionService>, job_store: &Arc<JobStore>) {
    crate::services::worker_status::begin("watcher/_franchise");
    let resp = collections.get_collections(true);
    match serde_json::to_string(&resp) {
        Ok(payload) => {
            job_store.save_collections_snapshot(&payload);
            crate::services::worker_status::ok(
                "watcher/_franchise",
                resp.summary.total_items as i64,
                "theo doi thay doi file",
            );
            log::info!(
                "[watcher] da luu collections_cache ({} muc)",
                resp.summary.total_items
            );
        }
        Err(e) => log::error!("[watcher] loi serialize collections: {}", e),
    }
}
