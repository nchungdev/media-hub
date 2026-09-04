use crate::domain::models::job::JobStatus;
use crate::services::aria2_service::Aria2Service;
use crate::services::job_store::JobStore;
use std::sync::Arc;
use std::time::Duration;

/// Vong lap nen xu ly hang doi tai xuong.
///
/// Nguon nao cung quy ve mot moi: aria2 nhan ca magnet lan https, nen worker
/// khong can biet job den tu TorBox hay tu mot link tai truc tiep. Cho nao can
/// biet la khi lay link -- viec do da lam truoc khi job duoc dua vao hang doi.
pub fn start(aria2: Arc<Aria2Service>, job_store: Arc<JobStore>) {
    // AppState::new() chay ngoai runtime Tokio nen khong dung tokio::spawn duoc
    // (se panic "no reactor running"). Tu dung runtime rieng trong thread nen.
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("[sync_worker] khong tao duoc runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            log::info!("[sync_worker] bat dau vong lap hang doi tai");
            loop {
                let alive = aria2.is_alive().await;
                let active = job_store.list_active().len() as i64;
                if alive {
                    crate::services::worker_status::begin("sync_worker");
                    tick(&aria2, &job_store).await;
                    crate::services::worker_status::ok(
                        "sync_worker",
                        active,
                        "aria2 RPC san sang",
                    );
                } else {
                    crate::services::worker_status::begin("sync_worker");
                    crate::services::worker_status::err(
                        "sync_worker",
                        "khong ket noi duoc aria2 RPC (aria2c --enable-rpc chua chay?)",
                    );
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    });
}

async fn tick(aria2: &Arc<Aria2Service>, job_store: &Arc<JobStore>) {
    let jobs = job_store.list_active();
    if jobs.is_empty() {
        return;
    }

    // Khong co daemon aria2 thi khoan dung toi hang doi, de nguyen trang thai
    // cho lan sau -- tranh danh dau that bai oan khi chi la daemon chua bat.
    if !aria2.is_alive().await {
        return;
    }

    for job in jobs {
        if job_store.is_cancel_requested(job.id) {
            if let Some(gid) = job_store.get_gid(job.id) {
                let _ = aria2.remove(&gid).await;
            }
            job_store.mark_cancelled(job.id);
            continue;
        }

        match job.status {
            JobStatus::Pending => {
                if job.source_uri.is_empty() {
                    job_store.mark_failed(job.id, "Job khong co nguon tai (source_uri rong)");
                    continue;
                }
                if job.staging_path.is_empty() {
                    job_store.mark_failed(job.id, "Job khong co thu muc dich (staging_path rong)");
                    continue;
                }

                match aria2.add_uri(&job.source_uri, &job.staging_path).await {
                    Ok(gid) => {
                        log::info!(
                            "[sync_worker] job #{} ({}) bat dau tai -> {}",
                            job.id,
                            job.franchise,
                            job.staging_path
                        );
                        job_store.mark_running(job.id, &gid, "Đang tải");
                    }
                    Err(e) => {
                        log::error!("[sync_worker] job #{} khong them duoc vao aria2: {}", job.id, e);
                        job_store.mark_failed(job.id, &format!("Không thêm được vào aria2: {}", e));
                    }
                }
            }

            JobStatus::Running => {
                let gid = match job_store.get_gid(job.id) {
                    Some(g) => g,
                    None => {
                        job_store.mark_failed(job.id, "Mất GID của aria2, không theo dõi được");
                        continue;
                    }
                };

                match aria2.tell_status(&gid).await {
                    Ok(p) => {
                        job_store.update_progress(
                            job.id,
                            p.completed_length,
                            p.total_length,
                            p.download_speed,
                        );

                        match p.status.as_str() {
                            "complete" => {
                                let file = p
                                    .files
                                    .first()
                                    .map(|f| {
                                        std::path::Path::new(f)
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| f.clone())
                                    })
                                    .unwrap_or_default();
                                log::info!("[sync_worker] job #{} tai xong: {}", job.id, file);
                                job_store.mark_done(job.id, &format!("Tải xong: {}", file));
                            }
                            "error" => {
                                let msg = if p.error_message.is_empty() {
                                    "aria2 báo lỗi không rõ".to_string()
                                } else {
                                    p.error_message.clone()
                                };
                                job_store.mark_failed(job.id, &msg);
                            }
                            "removed" => {
                                job_store.mark_cancelled(job.id);
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        // Mat dau vet GID (vd aria2 vua khoi dong lai) -> bao that bai
                        // de nguoi dung enqueue lai, thay vi treo mai o trang thai running.
                        log::warn!("[sync_worker] job #{} mat dau vet: {}", job.id, e);
                        job_store.mark_failed(job.id, &format!("Mất dấu vết tải: {}", e));
                    }
                }
            }

            _ => {}
        }
    }
}
