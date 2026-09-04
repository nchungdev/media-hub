use crate::services::agy_daemon::AgyDaemon;
use crate::services::job_store::JobStore;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

const WORKER: &str = "franchise_ai_classifier";
const BATCH_SIZE: usize = 15;
const ASK_TIMEOUT: Duration = Duration::from_secs(90);

/// Phan loai franchise bang agy cho nhung title khong co API nao tra duoc.
///
/// TMDb/TVDB chi co khai niem "collection" cho PHIM, khong co cho series --
/// nen nhung vu tru nhieu series/mua (Bay Vien Ngoc Rong / GT / Kai, moi
/// mua Super Sentai...) khong co cach nao tra tu dong. Worker nay hoi agy
/// suy luan tu ten phim, giong nhu mot bien tap vien xem qua danh sach.
///
/// Chay MOT LUOT theo yeu cau (nut Start trong tab Dich Vu), khong tu lap
/// lien tuc -- moi lan hoi ton token that. Sau khi xu ly het danh sach, tu
/// tat lai (enabled=false) de khong vo tinh chay mai.
pub fn start(agy: Arc<AgyDaemon>, job_store: Arc<JobStore>) {
    std::thread::spawn(move || {
        crate::services::worker_status::register(WORKER);
        crate::services::worker_status::set_enabled(WORKER, false);
        loop {
            if !crate::services::worker_status::is_enabled(WORKER) {
                crate::services::worker_status::sleep_interruptible(WORKER, Duration::from_secs(5));
                continue;
            }
            run_one_pass(&agy, &job_store);
            // Xong mot luot -> tu tat, cho nguoi dung bam Start lai khi can.
            // disable_silently() (khong phai set_enabled) de giu lai thong
            // diep tong ket run_one_pass() vua ghi, khong bi de thanh
            // "da dung theo yeu cau" chung chung.
            crate::services::worker_status::disable_silently(WORKER);
        }
    });
}

fn run_one_pass(agy: &Arc<AgyDaemon>, job_store: &Arc<JobStore>) {
    crate::services::worker_status::begin(WORKER);

    let status = agy.status();
    if status.get("running").and_then(|v| v.as_bool()) != Some(true) {
        crate::services::worker_status::err(WORKER, "daemon agy chưa sẵn sàng, thử lại sau");
        return;
    }

    let mut total_processed = 0i64;
    let mut total_found = 0i64;
    // Neu vong lap ben duoi dung som vi loi (vd het quota), giu lai thong
    // diep loi nay thay vi bi cau tong ket "hoan tat" chung chung de len tren.
    let mut broke_on_error: Option<String> = None;

    loop {
        let batch = job_store.list_uncached_for_ai(BATCH_SIZE);
        if batch.is_empty() {
            break;
        }
        if !crate::services::worker_status::is_enabled(WORKER) {
            // Nguoi dung bam Stop giua chung -> dung ngay, giu lai phan da lam.
            break;
        }

        let prompt = build_prompt(&batch);
        match agy.ask(&prompt, ASK_TIMEOUT) {
            Ok(text) => match parse_reply(&text) {
                Ok(map) => {
                    let mut results = Vec::new();
                    for (i, (root_key, _title, _mtype)) in batch.iter().enumerate() {
                        // Khop theo so thu tu (chuoi "1","2",...), khong khop theo
                        // ten phim -- ten co the chua dau ngoac/ky tu dac biet, va
                        // AI co the lap lai khong y het chuoi goc (thua/thieu dau
                        // cach, doi hoa-thuong...). So thu tu la doi mot, khong
                        // bao gio lech.
                        let franchise = map
                            .get(&(i + 1).to_string())
                            .cloned()
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        // Loc bo cau tra loi "null"/"none" chu, tranh AI tra chu thay vi JSON null.
                        let franchise = if is_negative(&franchise) {
                            String::new()
                        } else {
                            franchise
                        };
                        if !franchise.is_empty() {
                            total_found += 1;
                        }
                        results.push((root_key.clone(), franchise));
                    }
                    job_store.save_ai_franchise_batch(&results);
                    total_processed += results.len() as i64;

                    if let Err(e) = crate::services::library_aggregator::refresh_and_store(job_store) {
                        log::warn!("[{}] khong ghi duoc bang da gom: {}", WORKER, e);
                    }

                    crate::services::worker_status::ok(
                        WORKER,
                        total_processed,
                        &format!(
                            "đã xử lý {} title, tìm được {} franchise mới",
                            total_processed, total_found
                        ),
                    );
                }
                Err(e) => {
                    log::warn!("[{}] khong parse duoc phan hoi: {} -- noi dung: {}", WORKER, e, text);
                    // Van cache lai la "da hoi, khong ro" de khong hoi lai vo han lan
                    // that bai parse cung mot batch.
                    let results: Vec<_> = batch
                        .iter()
                        .map(|(k, _, _)| (k.clone(), String::new()))
                        .collect();
                    job_store.save_ai_franchise_batch(&results);
                    crate::services::worker_status::err(
                        WORKER,
                        &format!("lỗi đọc phản hồi AI: {}", e),
                    );
                }
            },
            Err(e) => {
                log::error!("[{}] agy.ask that bai: {}", WORKER, e);
                // Loi ket noi/timeout -> dung han vong nay, khong cache am de
                // lan chay sau thu lai dung batch nay.
                broke_on_error = Some(e);
                break;
            }
        }
    }

    let (total, found) = job_store.ai_cache_count();
    match broke_on_error {
        Some(e) => crate::services::worker_status::err(
            WORKER,
            &format!(
                "dừng giữa chừng sau khi xử lý {} title: {} · luỹ kế {} đã hỏi, {} có franchise",
                total_processed, e, total, found
            ),
        ),
        None => crate::services::worker_status::ok(
            WORKER,
            total_processed,
            &format!(
                "hoàn tất lượt này: {} title mới · luỹ kế {} đã hỏi, {} có franchise",
                total_processed, total, found
            ),
        ),
    }
}

fn build_prompt(batch: &[(String, String, String)]) -> String {
    let mut lines = String::new();
    for (i, (_key, title, mtype)) in batch.iter().enumerate() {
        let kind = if mtype == "movie" { "phim lẻ" } else { "series" };
        lines.push_str(&format!("{}. [{}] {}\n", i + 1, kind, title));
    }
    format!(
        "Đây là danh sách phim/series hoạt hình, đánh số thứ tự. Với MỖI \
mục, cho biết nó có thuộc một vũ trụ/franchise lớn hơn không -- tức là có \
phần khác (sequel, prequel, spin-off, các mùa khác nhau cùng nhân vật/thế \
giới) cũng nằm trong thư viện này hay không. Ví dụ: \"Bảy Viên Ngọc Rồng \
GT\" và \"Bảy Viên Ngọc Rồng Kai\" cùng thuộc franchise \"Bảy Viên Ngọc \
Rồng\"; \"Chiến Đội Nhẫn Phong Hurricanger\" thuộc franchise \"Super Sentai\".

Nếu KHÔNG chắc chắn hoặc phim thực sự độc lập (không có phần nào khác), \
trả về null cho mục đó -- đừng đoán bừa.

Danh sách:
{}

CHỈ trả lời bằng JSON hợp lệ, không thêm giải thích, không dùng markdown \
code fence. Khoá là SỐ THỨ TỰ dạng chuỗi (KHÔNG lặp lại tên phim), giá trị \
là tên franchise hoặc null. Ví dụ đúng: {{\"1\": \"Super Sentai\", \"2\": null}}",
        lines
    )
}

fn is_negative(s: &str) -> bool {
    let l = s.to_lowercase();
    l.is_empty()
        || l == "null"
        || l == "none"
        || l == "n/a"
        || l == "không"
        || l == "độc lập"
        || l.contains("không rõ")
}

/// agy doi khi boc JSON trong markdown fence hoac them van ban truoc/sau du
/// da yeu cau "chi JSON" -- cat tu dau "{" den cuoi "}" truoc khi parse.
fn parse_reply(text: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let start = text.find('{').ok_or("khong tim thay dau '{'")?;
    let end = text.rfind('}').ok_or("khong tim thay dau '}'")?;
    if end < start {
        return Err("dau ngoac khong hop le".into());
    }
    let json_str = &text[start..=end];
    let v: Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    let obj = v.as_object().ok_or("phan hoi khong phai JSON object")?;

    let mut map = std::collections::HashMap::new();
    for (k, val) in obj {
        let franchise = match val {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => continue,
        };
        map.insert(k.clone(), franchise);
    }
    Ok(map)
}
