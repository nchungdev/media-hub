/**
 * Unified Media Collections State & Filter Handlers
 */
import { showToast } from '../../core/toast.js';
import { renderCollectionCards } from './collection_renderer.js?v=2.6.1';

window.allMediaCollections = [];
window.allMediaFranchises = [];
window.currentColTypeFilter = 'all';
window.currentColSyncFilter = 'all';
window.currentColSubFilter = 'all';
window.currentColFranchiseFilter = 'all';
window.expandedCollectionId = null;
window.selectedCollectionSeason = {};
window.colPage = 1;
window.colPageSize = 12;
window.hasMoreCollections = false;
window.expandedFranchises = window.expandedFranchises || new Set();
window.currentFilteredCollections = [];

function escapeHtml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function escapeAttr(str) {
  if (!str) return '';
  return String(str)
    .replace(/'/g, "\\'")
    .replace(/"/g, '&quot;');
}

function applyCountsDetail(cd) {
  const set = (id, txt) => {
    const el = document.getElementById(id);
    if (el) el.innerText = txt;
  };
  if (!cd) return;
  for (const key of ['draft', 'jellyplex', 'drive']) {
    const c = cd[key] || { movies: 0, series: 0, total: 0 };
    set(`col-kpi-${key}`, c.total || 0);
    set(`col-kpi-${key}-detail`, `${c.movies || 0} Movies • ${c.series || 0} Series`);
  }
}

async function loadMediaCollections(forceRefresh = false) {
  const btnIcon = document.getElementById('btn-icon-col-refresh');
  if (btnIcon) btnIcon.classList.add('animate-spin');
  try {
    const res = await fetch(`/api/media/collections?refresh=${forceRefresh ? '1' : '0'}`);
    const data = await res.json();
    if (data && data.collections) {
      window.allMediaCollections = data.collections || [];
      window.allMediaFranchises = data.franchises || [];
      
      const sm = data.summary || {};
      if (document.getElementById('collection-kpi-badge')) {
        document.getElementById('collection-kpi-badge').innerText = `${sm.total_items || 0} Bộ Phim`;
      }
      if (document.getElementById('sidebar-collection-count')) {
        document.getElementById('sidebar-collection-count').innerText = sm.total_items || 0;
      }

      if (data.counts_detail) {
        applyCountsDetail(data.counts_detail);
      }
      populateFranchiseDropdown(window.allMediaCollections);
      loadStorageKpis();
      applyCollectionFilters();
    }
  } catch (e) {
    console.error("loadMediaCollections error:", e);
    loadStorageKpis();
  } finally {
    if (btnIcon) btnIcon.classList.remove('animate-spin');
  }
}

function populateFranchiseDropdown(collections) {
  const select = document.getElementById('col-franchise-select');
  if (!select) return;

  const currentVal = window.currentColFranchiseFilter || 'all';
  const franchiseCounts = new Map();
  let standaloneCount = 0;

  for (const c of (collections || [])) {
    const f = (c.franchise || '').trim();
    if (!f || f === 'Chưa phân loại') {
      standaloneCount++;
    } else {
      franchiseCounts.set(f, (franchiseCounts.get(f) || 0) + 1);
    }
  }

  // Sort by count descending, then alphabetical
  const sorted = Array.from(franchiseCounts.entries()).sort((a, b) => {
    if (b[1] !== a[1]) return b[1] - a[1];
    return a[0].localeCompare(b[0]);
  });

  let opts = `<option value="all">👑 Tất Cả Franchise (${sorted.length})</option>`;
  for (const [fName, cnt] of sorted) {
    opts += `<option value="${escapeAttr(fName)}">👑 ${escapeHtml(fName)} (${cnt})</option>`;
  }
  if (standaloneCount > 0) {
    opts += `<option value="__standalone__">🎬 Phim Độc Lập / Khác (${standaloneCount})</option>`;
  }

  select.innerHTML = opts;
  select.value = currentVal;
  if (select.value !== currentVal) {
    select.value = 'all';
    window.currentColFranchiseFilter = 'all';
  }
}

function setCollectionFranchiseFilter(fName) {
  window.currentColFranchiseFilter = fName || 'all';
  applyCollectionFilters();
}

/**
 * Ba thẻ kho lưu trữ: Draft (bản local chưa publish) / JellyPlex (Jellyfin
 * và Plex gộp lại vì cùng mô tả thư viện NAS) / Drive.
 * Số liệu lấy từ thư viện hợp nhất, đã khử trùng theo TMDb/TVDB id.
 */
async function loadStorageKpis() {
  try {
    const res = await fetch('/api/library/unified');
    const d = await res.json();
    if (d && d.counts_detail) {
      applyCountsDetail(d.counts_detail);
    }
  } catch (e) {
    console.error('loadStorageKpis error:', e);
  }
}

function setCollectionTypeFilter(type) {
  window.currentColTypeFilter = type;
  document.querySelectorAll('.col-type-btn').forEach(btn => {
    btn.className = "col-type-btn px-3 py-1.5 rounded-lg text-zinc-400 hover:text-white transition shrink-0";
  });
  const activeBtn = document.getElementById(`col-type-${type}`);
  if (activeBtn) activeBtn.className = "col-type-btn px-3 py-1.5 rounded-lg bg-amber-500 text-black font-bold transition shadow-sm shrink-0";
  applyCollectionFilters();
}

function setCollectionSyncFilter(filter) {
  window.currentColSyncFilter = filter;
  document.querySelectorAll('.col-sync-btn').forEach(btn => {
    btn.className = "col-sync-btn px-2.5 py-1 rounded-lg text-zinc-400 hover:text-white transition text-[11px] shrink-0";
  });
  const targetId = filter === 'synced_both' ? 'both' : (filter === 'only_nas' ? 'nas' : (filter === 'only_gdrive' ? 'drive' : (filter === 'unsynced' ? 'unsynced' : 'all')));
  const activeBtn = document.getElementById(`col-sync-${targetId}`);
  if (activeBtn) activeBtn.className = "col-sync-btn px-2.5 py-1 rounded-lg bg-zinc-800 text-white font-bold transition text-[11px] shrink-0";
  applyCollectionFilters();
}

function setCollectionSubFilter(filter) {
  window.currentColSubFilter = filter;
  document.querySelectorAll('.col-sub-btn').forEach(btn => {
    btn.className = "col-sub-btn px-2.5 py-1 rounded-lg text-zinc-400 hover:text-white transition text-[11px] shrink-0";
  });
  const activeBtn = document.getElementById(`col-sub-${filter}`);
  if (activeBtn) activeBtn.className = "col-sub-btn px-2.5 py-1 rounded-lg bg-zinc-800 text-white font-bold transition text-[11px] shrink-0";
  applyCollectionFilters();
}

function applyCollectionFilters() {
  const q = (document.getElementById('collection-search')?.value || '').toLowerCase().trim();
  let list = window.allMediaCollections || [];

  // Filter by search
  if (q) {
    list = list.filter(c => 
      (c.title || '').toLowerCase().includes(q) ||
      (c.vn_title || '').toLowerCase().includes(q) ||
      (c.franchise || '').toLowerCase().includes(q) ||
      (c.folder || '').toLowerCase().includes(q) ||
      (c.tvdb_id || '').includes(q)
    );
  }

  // Filter by franchise dropdown
  if (window.currentColFranchiseFilter && window.currentColFranchiseFilter !== 'all') {
    if (window.currentColFranchiseFilter === '__standalone__') {
      list = list.filter(c => {
        const f = (c.franchise || '').trim();
        return !f || f === 'Chưa phân loại';
      });
    } else {
      list = list.filter(c => (c.franchise || '').trim() === window.currentColFranchiseFilter);
    }
  }

  // Filter by type
  if (window.currentColTypeFilter !== 'all') {
    list = list.filter(c => c.type === window.currentColTypeFilter);
  }

  // Filter by sync state
  if (window.currentColSyncFilter !== 'all') {
    list = list.filter(c => c.sync.state === window.currentColSyncFilter);
  }

  // Filter by subtitle state
  if (window.currentColSubFilter !== 'all') {
    list = list.filter(c => c.subtitle.state === window.currentColSubFilter);
  }

  // Auto-expand groups when actively searching
  if (q) {
    window._searchExpanded = true;
    window.expandedFranchises = new Set(list.map(c => (c.franchise && c.franchise.trim() !== '') ? c.franchise.trim() : '__STANDALONE__'));
  } else if (window._searchExpanded) {
    window._searchExpanded = false;
    window.expandedFranchises = new Set();
  }

  // Auto-expand specific franchise if selected from dropdown
  if (window.currentColFranchiseFilter && window.currentColFranchiseFilter !== 'all') {
    window.expandedFranchises = window.expandedFranchises || new Set();
    window.expandedFranchises.add(window.currentColFranchiseFilter === '__standalone__' ? '__STANDALONE__' : window.currentColFranchiseFilter);
  }

  window.colPage = 1;
  window.currentFilteredCollections = list;
  renderCollectionCards(list);
}

function loadMoreCollections() {
  window.colPage = (window.colPage || 1) + 1;
  renderCollectionCards(window.currentFilteredCollections);
}

function toggleFranchiseCollapse(fName) {
  window.expandedFranchises = window.expandedFranchises || new Set();
  if (window.expandedFranchises.has(fName)) {
    window.expandedFranchises.delete(fName);
  } else {
    window.expandedFranchises.add(fName);
  }
  renderCollectionCards(window.currentFilteredCollections);
}

function collapseAllFranchises() {
  window.expandedFranchises = new Set();
  renderCollectionCards(window.currentFilteredCollections);
}

function expandAllFranchises() {
  window.expandedFranchises = new Set();
  if (window.currentGroupKeys) {
    window.currentGroupKeys.forEach(k => window.expandedFranchises.add(k));
  }
  renderCollectionCards(window.currentFilteredCollections);
}

function toggleCollectionDetail(colId) {
  if (window.expandedCollectionId === colId) {
    window.expandedCollectionId = null;
  } else {
    window.expandedCollectionId = colId;
  }
  renderCollectionCards(window.currentFilteredCollections);
}

function selectCollectionSeason(colId, sNum) {
  window.selectedCollectionSeason[colId] = sNum;
  renderCollectionCards(window.currentFilteredCollections);
}

// Global exposure for inline onclick handlers
window.applyCountsDetail = applyCountsDetail;
window.loadMediaCollections = loadMediaCollections;
window.loadStorageKpis = loadStorageKpis;
window.setCollectionTypeFilter = setCollectionTypeFilter;
window.setCollectionSyncFilter = setCollectionSyncFilter;
window.setCollectionSubFilter = setCollectionSubFilter;
window.applyCollectionFilters = applyCollectionFilters;
window.loadMoreCollections = loadMoreCollections;
window.toggleFranchiseCollapse = toggleFranchiseCollapse;
window.collapseAllFranchises = collapseAllFranchises;
window.expandAllFranchises = expandAllFranchises;
window.toggleCollectionDetail = toggleCollectionDetail;
window.selectCollectionSeason = selectCollectionSeason;
window.setCollectionFranchiseFilter = setCollectionFranchiseFilter;

export {
  loadMediaCollections,
  loadStorageKpis,
  applyCountsDetail,
  setCollectionTypeFilter,
  setCollectionSyncFilter,
  setCollectionSubFilter,
  setCollectionFranchiseFilter,
  applyCollectionFilters,
  loadMoreCollections,
  toggleFranchiseCollapse,
  collapseAllFranchises,
  expandAllFranchises,
  toggleCollectionDetail,
  selectCollectionSeason
};

