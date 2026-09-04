use crate::services::job_store::JobStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedItem {
    /// Khoa doi chieu chung: "tvdb-123" / "tmdb-456" / "name-..."
    pub media_key: String,
    pub title: String,
    pub franchise: String,
    #[serde(rename = "type")]
    pub media_type: String,
    pub in_local: bool,
    pub in_nas: bool,
    pub in_gdrive: bool,
    /// Ten thu muc tai tung noi (co the khac nhau giua 3 nguon).
    pub folders: HashMap<String, String>,
    pub paths: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFranchise {
    pub name: String,
    pub total: usize,
    pub only_local: usize,
    pub only_remote: usize,
    pub everywhere: usize,
    pub items: Vec<UnifiedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLibrary {
    pub franchises: Vec<UnifiedFranchise>,
    pub total_items: usize,
    pub counts_by_source: HashMap<String, usize>,
    pub unclassified: usize,
}

/// Nhan cho nhung muc chi ton tai tren NAS/Drive: chung phang theo chuan Plex
/// nen khong tu khai bao duoc franchise, va cung khong co ban local de muon.
pub const UNCLASSIFIED: &str = "Chưa phân loại";

pub fn aggregate(job_store: &Arc<JobStore>) -> UnifiedLibrary {
    let rows = job_store.load_library_index();

    // Ban do media_key -> franchise.
    // Uu tien local (thu muc do chinh nguoi dung sap xep), sau do moi den
    // BoxSet cua Jellyfin -- nho vay nhung title chi co tren NAS van co
    // franchise thay vi roi hết vao nhom "Chua phan loai".
    let mut franchise_of: HashMap<String, String> = HashMap::new();
    for (source, key, franchise, _t, _f, _mt, _p) in &rows {
        if franchise.is_empty() {
            continue;
        }
        match source.as_str() {
            "local" => {
                franchise_of.insert(key.clone(), franchise.clone());
            }
            _ => {
                franchise_of
                    .entry(key.clone())
                    .or_insert_with(|| franchise.clone());
            }
        }
    }
    // Chay lai vong nua de local luon thang neu co ca hai.
    for (source, key, franchise, _t, _f, _mt, _p) in &rows {
        if source == "local" && !franchise.is_empty() {
            franchise_of.insert(key.clone(), franchise.clone());
        }
    }

    let mut items: HashMap<String, UnifiedItem> = HashMap::new();
    let mut counts_by_source: HashMap<String, usize> = HashMap::new();

    for (source, key, _franchise, title, folder, media_type, path) in &rows {
        *counts_by_source.entry(source.clone()).or_insert(0) += 1;

        let it = items.entry(key.clone()).or_insert_with(|| UnifiedItem {
            media_key: key.clone(),
            title: title.clone(),
            franchise: franchise_of
                .get(key)
                .cloned()
                .unwrap_or_else(|| UNCLASSIFIED.to_string()),
            media_type: media_type.clone(),
            in_local: false,
            in_nas: false,
            in_gdrive: false,
            folders: HashMap::new(),
            paths: HashMap::new(),
        });

        match source.as_str() {
            "local" => it.in_local = true,
            "nas" => it.in_nas = true,
            "gdrive" => it.in_gdrive = true,
            _ => {}
        }
        it.folders.insert(source.clone(), folder.clone());
        it.paths.insert(source.clone(), path.clone());

        // Ban local uu tien lam ten hien thi vi da duoc chuan hoa ky nhat.
        if source == "local" && !title.is_empty() {
            it.title = title.clone();
        }
    }

    // Gom theo franchise
    let mut by_franchise: HashMap<String, Vec<UnifiedItem>> = HashMap::new();
    for it in items.into_values() {
        by_franchise
            .entry(it.franchise.clone())
            .or_default()
            .push(it);
    }

    let mut franchises: Vec<UnifiedFranchise> = by_franchise
        .into_iter()
        .map(|(name, mut items)| {
            items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            let only_local = items
                .iter()
                .filter(|i| i.in_local && !i.in_nas && !i.in_gdrive)
                .count();
            let only_remote = items.iter().filter(|i| !i.in_local).count();
            let everywhere = items
                .iter()
                .filter(|i| i.in_local && i.in_nas && i.in_gdrive)
                .count();
            UnifiedFranchise {
                name,
                total: items.len(),
                only_local,
                only_remote,
                everywhere,
                items,
            }
        })
        .collect();

    // Franchise thuc su len truoc, nhom chua phan loai xuong cuoi.
    franchises.sort_by(|a, b| {
        let a_un = a.name == UNCLASSIFIED;
        let b_un = b.name == UNCLASSIFIED;
        a_un.cmp(&b_un)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let total_items = franchises.iter().map(|f| f.total).sum();
    let unclassified = franchises
        .iter()
        .find(|f| f.name == UNCLASSIFIED)
        .map(|f| f.total)
        .unwrap_or(0);

    UnifiedLibrary {
        franchises,
        total_items,
        counts_by_source,
        unclassified,
    }
}
