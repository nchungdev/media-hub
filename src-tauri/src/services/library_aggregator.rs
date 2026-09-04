use crate::services::job_store::JobStore;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedItem {
    /// Khoa doi chieu chung: "tvdb-123" / "tmdb-456" / "name-..."
    pub media_key: String,
    pub title: String,
    pub franchise: String,
    #[serde(rename = "type")]
    pub media_type: String,
    /// Draft = ban local trong .media-hub, chua publish len NAS/Drive.
    pub in_draft: bool,
    /// Da publish len NAS (Jellyfin va/hoac Plex nhin thay).
    pub in_nas: bool,
    pub in_drive: bool,
    /// Nguon nao xac nhan co mat: draft / jellyfin / plex / drive
    pub seen_by: Vec<String>,
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
    /// Franchise chi co dung mot title (phim le).
    pub is_single: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLibrary {
    pub franchises: Vec<UnifiedFranchise>,
    pub total_items: usize,
    pub counts_by_source: HashMap<String, usize>,
    /// Tach Movies/Series cho tung kho: draft / jellyplex / drive.
    /// JellyPlex la ten goi gop cho Jellyfin va Plex -- ca hai cung mo ta
    /// thu vien NAS nen giao dien chi hien thanh mot kho duy nhat.
    pub counts_detail: HashMap<String, SourceCount>,
    pub unclassified: usize,
}

pub use crate::domain::models::collection::SourceCount;

/// Nhan cho nhung muc chi ton tai tren NAS/Drive: chung phang theo chuan Plex
/// nen khong tu khai bao duoc franchise, va cung khong co ban local de muon.
pub const UNCLASSIFIED: &str = "Chưa phân loại";

/// Union-find de gop cac media_key cung tro toi mot title.
struct Dsu {
    parent: HashMap<String, String>,
}

impl Dsu {
    fn new() -> Self {
        Self { parent: HashMap::new() }
    }
    fn find(&mut self, x: &str) -> String {
        let p = self.parent.entry(x.to_string()).or_insert_with(|| x.to_string()).clone();
        if p == x {
            return p;
        }
        let root = self.find(&p);
        self.parent.insert(x.to_string(), root.clone());
        root
    }
    fn union(&mut self, a: &str, b: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent.insert(ra, rb);
        }
    }
}

pub fn aggregate(job_store: &Arc<JobStore>) -> UnifiedLibrary {
    aggregate_with_keymap(job_store).0
}

/// Nhu aggregate() nhung tra ve them ban do media_key -> khoa goc.
/// Ban do nay chi ton tai ben trong union-find; khong tra ra thi khong the
/// tra cuu bang khoa thanh phan (vd imdb-tt123 cua mot phim co khoa goc tmdb-456).
pub fn aggregate_with_keymap(
    job_store: &Arc<JobStore>,
) -> (UnifiedLibrary, HashMap<String, String>) {
    let rows = job_store.load_library_index();

    // Buoc 1: noi cac media_key thuoc cung mot title.
    // Mot phim thuong mang nhieu id (tmdb + tvdb + imdb); neu gom theo tung
    // key rieng thi mot phim bi dem thanh nhieu muc. Cac dong cung
    // (source, item_uid) chac chan la cung mot phim -> hop nhat khoa cua chung.
    let mut dsu = Dsu::new();
    let mut by_item: HashMap<(String, String), Vec<String>> = HashMap::new();
    for (source, key, _f, _t, _fo, _mt, _p, item_uid) in &rows {
        if item_uid.is_empty() {
            continue;
        }
        by_item
            .entry((source.clone(), item_uid.clone()))
            .or_default()
            .push(key.clone());
    }
    for keys in by_item.values() {
        for w in keys.windows(2) {
            dsu.union(&w[0], &w[1]);
        }
    }

    // Buoc 2: franchise theo tung nhom (uu tien local, roi den BoxSet/TMDb).
    let mut franchise_of: HashMap<String, String> = HashMap::new();
    for (source, key, franchise, _t, _fo, _mt, _p, _u) in &rows {
        if franchise.is_empty() {
            continue;
        }
        let root = dsu.find(key);
        if source == "local" {
            franchise_of.insert(root, franchise.clone());
        } else {
            franchise_of.entry(root).or_insert_with(|| franchise.clone());
        }
    }

    let mut items: HashMap<String, UnifiedItem> = HashMap::new();
    let mut counts_by_source: HashMap<String, usize> = HashMap::new();
    let mut seen_title: HashMap<String, HashSet<String>> = HashMap::new();

    for (source, key, _franchise, title, folder, media_type, path, _u) in &rows {
        let root = dsu.find(key);
        seen_title
            .entry(source.clone())
            .or_default()
            .insert(root.clone());

        let it = items.entry(root.clone()).or_insert_with(|| UnifiedItem {
            media_key: root.clone(),
            title: title.clone(),
            // Khong thuoc collection nao -> chinh no la mot franchise don le,
            // thay vi don het vao mot ro "chua phan loai" khong co y nghia.
            franchise: franchise_of
                .get(&root)
                .cloned()
                .unwrap_or_else(|| title.clone()),
            media_type: media_type.clone(),
            in_draft: false,
            in_nas: false,
            in_drive: false,
            seen_by: Vec::new(),
            folders: HashMap::new(),
            paths: HashMap::new(),
        });

        match source.as_str() {
            "local" => it.in_draft = true,
            // Jellyfin va Plex deu mo ta cung thu vien NAS.
            "jellyfin" | "plex" => it.in_nas = true,
            "gdrive" => it.in_drive = true,
            _ => {}
        }
        if !it.seen_by.contains(source) {
            it.seen_by.push(source.clone());
        }
        it.folders.insert(source.clone(), folder.clone());
        if !path.is_empty() {
            it.paths.insert(source.clone(), path.clone());
        }
        if source == "local" && !title.is_empty() {
            it.title = title.clone();
        }
    }

    // Dem theo nguon: so TITLE rieng biet, khong phai so dong (mot title
    // mang nhieu id se sinh nhieu dong).
    for (src, roots) in &seen_title {
        counts_by_source.insert(src.clone(), roots.len());
    }

    // Tach Movies/Series cho tung kho. Jellyfin va Plex gop lam mot ("jellyplex")
    // vi ca hai cung mo ta thu vien NAS -- tach ra chi lam nguoi dung roi.
    let mut counts_detail: HashMap<String, SourceCount> = HashMap::new();
    for (src, roots) in &seen_title {
        let bucket = match src.as_str() {
            "local" => "draft",
            "jellyfin" | "plex" => "jellyplex",
            "gdrive" => "drive",
            _ => continue,
        };
        let entry = counts_detail.entry(bucket.to_string()).or_default();
        for root in roots {
            if let Some(it) = items.get(root) {
                if it.media_type == "movie" {
                    entry.movies += 1;
                } else {
                    entry.series += 1;
                }
            }
        }
    }
    // Jellyfin + Plex cong don se dem trung title ma ca hai cung thay,
    // nen dem lai theo tap hop hop nhat.
    {
        let mut nas_roots: HashSet<String> = HashSet::new();
        for src in ["jellyfin", "plex"] {
            if let Some(r) = seen_title.get(src) {
                nas_roots.extend(r.iter().cloned());
            }
        }
        let mut c = SourceCount::default();
        for root in &nas_roots {
            if let Some(it) = items.get(root) {
                if it.media_type == "movie" {
                    c.movies += 1;
                } else {
                    c.series += 1;
                }
            }
        }
        c.total = nas_roots.len();
        counts_detail.insert("jellyplex".to_string(), c);
    }
    for (_k, v) in counts_detail.iter_mut() {
        if v.total == 0 {
            v.total = v.movies + v.series;
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
                .filter(|i| i.in_draft && !i.in_nas && !i.in_drive)
                .count();
            let only_remote = items.iter().filter(|i| !i.in_draft).count();
            let everywhere = items
                .iter()
                .filter(|i| i.in_draft && i.in_nas && i.in_drive)
                .count();
            UnifiedFranchise {
                name,
                total: items.len(),
                only_local,
                only_remote,
                everywhere,
                is_single: items.len() == 1,
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

    // Ban do day du: moi khoa da gap -> khoa goc cua nhom.
    let mut key_map: HashMap<String, String> = HashMap::new();
    for (_s, key, _f, _t, _fo, _mt, _p, _u) in &rows {
        let root = dsu.find(key);
        key_map.insert(key.clone(), root);
    }

    (
        UnifiedLibrary {
            franchises,
            total_items,
            counts_by_source,
            counts_detail,
            unclassified,
        },
        key_map,
    )
}

/// Gom lai tu library_index roi ghi ket qua xuong DB.
/// Goi sau moi lan mot indexer chay xong, thay vi hen gio -- de bang da gom
/// luon phan anh dung trang thai moi nhat cua ca 3 nguon.
pub fn refresh_and_store(job_store: &Arc<JobStore>) -> Result<usize, String> {
    let (lib, key_map) = aggregate_with_keymap(job_store);

    let mut items = Vec::new();
    for fr in &lib.franchises {
        for it in &fr.items {
            items.push((
                it.media_key.clone(),
                it.title.clone(),
                it.franchise.clone(),
                it.media_type.clone(),
                it.in_draft,
                it.in_nas,
                it.in_drive,
                serde_json::to_string(&it.seen_by).unwrap_or_else(|_| "[]".into()),
                serde_json::to_string(&it.folders).unwrap_or_else(|_| "{}".into()),
                serde_json::to_string(&it.paths).unwrap_or_else(|_| "{}".into()),
            ));
        }
    }

    let pairs: Vec<(String, String)> = key_map.into_iter().collect();
    job_store.save_unified(&items, &pairs)
}
