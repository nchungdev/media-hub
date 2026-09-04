/**
 * Torbox Torrent List & Filter Management
 */
import { showToast } from '../../core/toast.js';

                    let currentEngineFilter = "all";
    let currentTorboxFilter = "all";

    function setEngineFilter(engine) {
      currentEngineFilter = engine;
      document.querySelectorAll('.engine-tab-btn').forEach(btn => {
        btn.className = "engine-tab-btn px-3 py-1.5 rounded-lg text-zinc-400 hover:text-white transition shrink-0";
      });
      const activeEngineBtn = document.getElementById(`engine-btn-${engine}`);
      if (activeEngineBtn) {
        activeEngineBtn.className = "engine-tab-btn px-3 py-1.5 rounded-lg bg-purple-600 text-white font-bold transition shadow-sm shrink-0";
      }
      applyTorboxFilter();
    }

    function filterTorboxStatus(status) {
      currentTorboxFilter = status;
      document.querySelectorAll('.torbox-filter-btn').forEach(btn => {
        btn.className = "torbox-filter-btn px-3 py-1.5 rounded-lg text-zinc-400 hover:text-white transition shrink-0";
      });
      const activeStatusBtn = document.getElementById(`tbfilter-${status}`);
      if (activeStatusBtn) {
        if (status === 'cached') {
          activeStatusBtn.className = "torbox-filter-btn px-3 py-1.5 rounded-lg bg-emerald-600 text-white font-bold transition shadow-sm shrink-0";
        } else if (status === 'active') {
          activeStatusBtn.className = "torbox-filter-btn px-3 py-1.5 rounded-lg bg-blue-600 text-white font-bold transition shadow-sm shrink-0";
        } else if (status === 'queued') {
          activeStatusBtn.className = "torbox-filter-btn px-3 py-1.5 rounded-lg bg-purple-600 text-white font-bold transition shadow-sm shrink-0";
        } else {
          activeStatusBtn.className = "torbox-filter-btn px-3 py-1.5 rounded-lg bg-zinc-800 text-white font-bold transition shadow-sm shrink-0";
        }
      }
      applyTorboxFilter();
    }

    async function clearTorboxCache() {
      const icon = document.getElementById('btn-icon-torbox-cache');
      if (icon) icon.classList.add('animate-spin');
      try {
        const res = await fetch('/api/torbox/clear_cache', { method: 'POST' });
        const data = await res.json();
        if (data.data && Array.isArray(data.data)) {
          currentTorrents = data.data;
          applyTorboxFilter();
          const tbCount = document.getElementById('torbox-count');
          if (tbCount) {
            tbCount.innerText = `${currentTorrents.length} Downloads`;
          }
          showToast(`⚡ Đã làm mới ${currentTorrents.length} tác vụ tải!`, 'success');
        } else {
          const res2 = await fetch('/api/torbox?refresh=true');
          const data2 = await res2.json();
          if (data2.data) {
            currentTorrents = data2.data;
            applyTorboxFilter();
            showToast(`⚡ Đã làm mới ${currentTorrents.length} tác vụ tải!`, 'success');
          }
        }
      } catch (err) {
        showToast('Lỗi khi xoá cache: ' + err.message, 'error');
      } finally {
        if (icon) icon.classList.remove('animate-spin');
      }
    }

    function filterTorbox() {
      applyTorboxFilter();
    }

    function applyTorboxFilter() {
      const q = (document.getElementById('torbox-search') ? document.getElementById('torbox-search').value : '').toLowerCase();
      let filtered = currentTorrents.filter(t => (t.name || '').toLowerCase().includes(q) || (t.id || '').toString().includes(q));

      // Filter by Engine
      if (currentEngineFilter === 'torbox') {
        filtered = filtered.filter(t => !t.engine || t.engine === 'torbox');
      } else if (currentEngineFilter === 'aria2') {
        filtered = filtered.filter(t => t.engine === 'aria2');
      } else if (currentEngineFilter === 'direct') {
        filtered = filtered.filter(t => t.engine === 'direct');
      }

      // Calculate pure download counts
      const countCached = currentTorrents.filter(t => t.progress >= 1 || t.download_state === 'completed' || t.cached).length;
      const countActive = currentTorrents.filter(t => t.download_state === 'downloading' || (t.progress > 0 && t.progress < 1)).length;
      const countQueued = currentTorrents.filter(t => t.download_state === 'queued' || t.is_queued || t.progress === 0).length;

      if (document.getElementById('tbcount-all')) document.getElementById('tbcount-all').innerText = currentTorrents.length;
      if (document.getElementById('tbcount-cached')) document.getElementById('tbcount-cached').innerText = countCached;
      if (document.getElementById('tbcount-active')) document.getElementById('tbcount-active').innerText = countActive;
      if (document.getElementById('tbcount-queued')) document.getElementById('tbcount-queued').innerText = countQueued;

      // Filter by Download Status
      if (currentTorboxFilter === 'cached') {
        filtered = filtered.filter(t => t.progress >= 1 || t.download_state === 'completed' || t.cached);
      } else if (currentTorboxFilter === 'active') {
        filtered = filtered.filter(t => t.download_state === 'downloading' || (t.progress > 0 && t.progress < 1));
      } else if (currentTorboxFilter === 'queued') {
        filtered = filtered.filter(t => t.download_state === 'queued' || t.is_queued || t.progress === 0);
      }

      renderTorbox(filtered);
    }

        // ==================== SMART MULTI-SOURCE SYNC ENGINE ====================
        async function fetchTorrents() {
      try {
        const res = await fetch('/api/torbox');
        const data = await res.json();
        if (data.data) {
          currentTorrents = data.data;
          applyTorboxFilter();
        }
      } catch (e) {
        console.error("fetchTorrents error:", e);
      }
    }

        // Queue a real job. The server de-duplicates by torrent id, so asking for the same
    // torrent twice adds a destination instead of downloading it a second time.

    function renderTorbox(items) {
      const tbody = document.getElementById('torbox-tbody');
      if (!tbody) return;

      if (!items || items.length === 0) {
        tbody.innerHTML = `
          <tr>
            <td colspan="5" class="px-4 py-8 text-center text-zinc-500 text-xs">
              Không tìm thấy torrent nào phù hợp với bộ lọc.
            </td>
          </tr>
        `;
        return;
      }

      tbody.innerHTML = items.map(t => {
        const isQueued = t.is_queued || t.download_state === 'queued';
        const isCompleted = t.progress >= 1 || t.download_state === 'completed' || t.cached;
        const sizeGb = t.size ? (t.size / (1024*1024*1024)).toFixed(2) : '0.00';
        const safeName = (t.name || '').replace(/\\/g, '\\\\').replace(/'/g, "\\'");
        
        let statusBadge = '';
        if (isCompleted) {
          statusBadge = `<span class="px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 font-bold text-[10px] whitespace-nowrap flex items-center gap-1.5 justify-center"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ⚡ Cached Cloud</span>`;
        } else if (t.download_state === 'downloading' || (t.progress > 0 && t.progress < 1)) {
          const pct = Math.round((t.progress || 0) * 100);
          statusBadge = `<span class="px-2.5 py-1 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/30 font-bold text-[10px] animate-pulse whitespace-nowrap flex items-center gap-1.5 justify-center"><span class="w-1.5 h-1.5 rounded-full bg-blue-400 animate-ping"></span> ⚡ Đang Kéo ${pct}%</span>`;
        } else if (isQueued) {
          statusBadge = `<span class="px-2 py-0.5 rounded-lg bg-zinc-800 text-zinc-400 border border-zinc-700 font-medium text-[10px] whitespace-nowrap">⏳ Hàng Đợi</span>`;
        } else {
          statusBadge = `<span class="px-2 py-0.5 rounded-lg bg-zinc-800/80 text-zinc-400 border border-zinc-700/60 font-medium text-[10px] whitespace-nowrap">Sẵn Sàng</span>`;
        }

        return `
          <tr class="hover:bg-zinc-800/30 transition">
            <td class="px-3 py-3 text-center w-10">
              <input type="checkbox" value="${t.id}" class="torbox-item-cb w-4 h-4 rounded border-zinc-700 bg-zinc-900 text-purple-600 focus:ring-0 cursor-pointer" onchange="updateTorboxSelection()">
            </td>
            <td class="px-3 py-3 w-60 sm:w-72 lg:w-80">
              <div class="font-bold text-white text-xs truncate max-w-[220px] sm:max-w-[280px] lg:max-w-[320px]" title="${t.name}">${t.name}</div>
              <div class="text-[10px] text-zinc-500 font-mono mt-0.5 flex items-center gap-1.5">
                <span>${t.engine ? t.engine.toUpperCase() : 'TORBOX'}</span>
                <span>• #${t.id}</span>
              </div>
            </td>
            <td class="px-3 py-3 font-mono text-zinc-300 font-semibold w-24 whitespace-nowrap">${isQueued ? 'Chờ slot' : sizeGb + ' GB'}</td>
            <td class="px-3 py-3 w-44 whitespace-nowrap text-center">
              ${statusBadge}
            </td>
            <td class="px-3 py-3 text-right whitespace-nowrap">
              <div class="flex items-center justify-end gap-1.5 whitespace-nowrap">
                <!-- Direct DDL Download Button -->
                <button onclick="downloadTorboxTorrent(${t.id}, '${safeName}')" class="px-2.5 py-1 bg-purple-600/10 hover:bg-purple-600 text-purple-300 hover:text-white border border-purple-500/30 text-xs font-semibold rounded-xl transition flex items-center gap-1 shadow-sm shrink-0" title="Tải file trực tiếp về máy">
                  <span>📥</span> Tải Về
                </button>

                <!-- Copy Link Button -->
                <button onclick="copyTorboxDownloadLink(${t.id})" class="px-2.5 py-1 bg-zinc-900 hover:bg-zinc-800 border border-zinc-800 text-zinc-400 hover:text-white text-xs font-medium rounded-xl transition flex items-center gap-1 shadow-sm shrink-0" title="Copy Link Tải Trực Tiếp">
                  <span>📋</span> Link
                </button>

                <!-- Delete Button -->
                <button onclick="deleteTorrent(${t.id})" class="px-2 py-1 text-zinc-500 hover:text-red-400 hover:bg-red-500/10 border border-transparent hover:border-red-500/20 transition rounded-xl text-xs font-medium shrink-0" title="Xóa tác vụ tải">
                  🗑️
                </button>
              </div>
            </td>
          </tr>
        `;
      }).join('');
      updateTorboxSelection();
    }



export {
  setEngineFilter,
  filterTorboxStatus,
  clearTorboxCache,
  filterTorbox,
  applyTorboxFilter,
  fetchTorrents,
  renderTorbox
};
