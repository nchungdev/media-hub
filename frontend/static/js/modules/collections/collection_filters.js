/**
 * Unified Media Collections State & Filter Handlers
 */
import { showToast } from '../../core/toast.js';

window.currentColTypeFilter = 'all';
window.currentColSyncFilter = 'all';
window.currentColSubFilter = 'all';
window.expandedCollectionId = null;
window.selectedCollectionSeason = {};

    // ==================== BỘ SƯU TẬP MEDIA (UNIFIED MEDIA COLLECTIONS) ====================
    window.allMediaCollections = [];
    window.currentColTypeFilter = 'all';
    window.currentColSyncFilter = 'all';
    window.currentColSubFilter = 'all';
    window.expandedCollectionId = null;
    window.selectedCollectionSeason = {};

    async function loadMediaCollections(forceRefresh = false) {
      const btnIcon = document.getElementById('btn-icon-col-refresh');
      if (btnIcon) btnIcon.classList.add('animate-spin');
      try {
        const res = await fetch(`/api/media/collections?refresh=${forceRefresh ? '1' : '0'}`);
        const data = await res.json();
        if (data && data.collections) {
          window.allMediaCollections = data.collections;
          
          const sm = data.summary || {};
          if (document.getElementById('collection-kpi-badge')) document.getElementById('collection-kpi-badge').innerText = `${sm.total_items || 0} Bộ Phim`;
          if (document.getElementById('sidebar-collection-count')) document.getElementById('sidebar-collection-count').innerText = sm.total_items || 0;

          loadStorageKpis();
          applyCollectionFilters();
        }
      } catch (e) {
        console.error("loadMediaCollections error:", e);
      } finally {
        if (btnIcon) btnIcon.classList.remove('animate-spin');
      }
    }

    /**
     * Ba thẻ kho lưu trữ: Draft (bản local chưa publish) / JellyPlex (Jellyfin
     * và Plex gộp lại vì cùng mô tả thư viện NAS) / Drive.
     * Số liệu lấy từ thư viện hợp nhất, đã khử trùng theo TMDb/TVDB id.
     */
    async function loadStorageKpis() {
      const set = (id, txt) => {
        const el = document.getElementById(id);
        if (el) el.innerText = txt;
      };
      try {
        const res = await fetch('/api/library/unified');
        const d = await res.json();
        const cd = d.counts_detail || {};
        for (const key of ['draft', 'jellyplex', 'drive']) {
          const c = cd[key] || { movies: 0, series: 0, total: 0 };
          set(`col-kpi-${key}`, c.total || 0);
          set(`col-kpi-${key}-detail`, `${c.movies || 0} Movies • ${c.series || 0} Series`);
        }
      } catch (e) {
        console.error('loadStorageKpis error:', e);
        for (const key of ['draft', 'jellyplex', 'drive']) {
          set(`col-kpi-${key}`, '--');
          set(`col-kpi-${key}-detail`, 'không tải được');
        }
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
          (c.folder || '').toLowerCase().includes(q) ||
          (c.tvdb_id || '').includes(q)
        );
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

      renderCollectionCards(list);
    }

    function toggleCollectionDetail(colId) {
      if (window.expandedCollectionId === colId) {
        window.expandedCollectionId = null;
      } else {
        window.expandedCollectionId = colId;
      }
      applyCollectionFilters();
    }

    function selectCollectionSeason(colId, sNum) {
      window.selectedCollectionSeason[colId] = sNum;
      applyCollectionFilters();
    }

    function toggleEpisodeSubtitles(colId, epKey) {
      const row = document.getElementById(`subfiles-${colId}-${epKey}`);
      const arrow = document.getElementById(`arrow-sub-${colId}-${epKey}`);
      if (row) {
        row.classList.toggle('hidden');
        if (arrow) {
          arrow.innerText = row.classList.contains('hidden') ? '▾' : '▴';
        }
      }
    }


export {
  loadMediaCollections,
  loadStorageKpis,
  setCollectionTypeFilter,
  setCollectionSyncFilter,
  setCollectionSubFilter,
  applyCollectionFilters,
  toggleCollectionDetail,
  selectCollectionSeason,
  toggleEpisodeSubtitles
};
