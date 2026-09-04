/**
 * Media Toolbox, TMDb Lookup, NAS Scanner & Library Index Builder
 */
import { showToast } from '../../core/toast.js';

    async function scanNasLibraries() {
      const host = document.getElementById('cfg-nas-host')?.value?.trim();
      const user = document.getElementById('cfg-nas-user')?.value?.trim() || 'admin';
      const port = document.getElementById('cfg-nas-port')?.value || 22;

      if (!host) {
        showToast('Vui lòng nhập địa chỉ IP của NAS trước', 'warning');
        return;
      }

      const resBox = document.getElementById('nas-scan-results');
      if (resBox) {
        resBox.classList.remove('hidden');
        resBox.innerHTML = '<div class="text-amber-400 animate-pulse">⏳ Đang kết nối SSH và quét các thư mục Plex trên NAS...</div>';
      }

      const key = document.getElementById('cfg-nas-ssh-key')?.value.trim() || '';
      const customPath = document.getElementById('cfg-nas-path')?.value.trim() || '';

      try {
        const res = await fetch('/api/nas/scan', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({host, user, port, key, path: customPath})
        });
        const data = await res.json();
        if (data.success && data.libraries && data.libraries.length > 0) {
          showToast(`Tìm thấy ${data.libraries.length} thư mục Plex trên NAS!`, 'success');
          if (resBox) {
            resBox.innerHTML = `
              <div class="font-bold text-emerald-400">✅ Đã phát hiện các thư mục Plex trên NAS:</div>
              <div class="flex flex-wrap gap-1.5 pt-1">
                ${data.libraries.map(p => `
                  <button type="button" onclick="document.getElementById('cfg-nas-path').value='${p}'" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 border border-zinc-700 text-[11px] font-mono text-zinc-300 hover:text-white transition">
                    📁 ${p}
                  </button>
                `).join('')}
              </div>
            `;
          }
        } else {
          if (resBox) resBox.innerHTML = `<div class="text-zinc-500">⚪ Không tìm thấy thư mục Plex mặc định hoặc kết nối bị từ chối (${data.error || 'Timeout'})</div>`;
        }
      } catch (e) {
        if (resBox) resBox.innerHTML = `<div class="text-red-400">❌ Lỗi: ${e}</div>`;
      }
    }

    async function checkGDriveConnection() {
      const remote = document.getElementById('cfg-gdrive-remote')?.value || 'gdrive';
      const root = document.getElementById('cfg-gdrive-root')?.value || 'Phim/TV Shows';

      showToast('⏳ Đang kiểm tra kết nối Rclone Google Drive...', 'info');
      try {
        const res = await fetch('/api/gdrive/check', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({remote, root})
        });
        const data = await res.json();
        if (data.success) {
          showToast('✅ ' + data.message, 'success');
        } else {
          showToast('❌ ' + (data.error || 'Lỗi kết nối Drive'), 'error');
        }
      } catch (e) {
        showToast('Lỗi: ' + e, 'error');
      }
    }

    async function purgeStagingManual() {
      if (!confirm('Bạn có chắc chắn muốn dọn dẹp sạch toàn bộ file đệm tạm thời trong thư mục media_staging?')) {
        return;
      }
      showToast('⏳ Đang quét và giải phóng bộ đệm...', 'info');
      try {
        const res = await fetch('/api/staging/purge', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({})
        });
        const data = await res.json();
        if (data.success) {
          showToast('🧹 ' + data.message, 'success');
        } else {
          showToast('❌ ' + (data.error || 'Lỗi khi dọn dẹp'), 'error');
        }
      } catch (e) {
        showToast('Lỗi: ' + e, 'error');
      }
    }

    // ==================== TOOLBOX CONTROLLER FUNCTIONS ====================
    async function inspectCollectorSource() {
      const input = document.getElementById('collector-input')?.value?.trim();
      if (!input) {
        showToast('Vui lòng nhập Magnet link hoặc tên phim', 'warning');
        return;
      }
      const resBox = document.getElementById('collector-results');
      if (resBox) {
        resBox.classList.remove('hidden');
        resBox.innerHTML = '<div class="text-blue-400 animate-pulse">⏳ Đang phân tích metadata nguồn và sơ đồ phân đoạn...</div>';
      }
      try {
        const res = await fetch('/api/collector/inspect', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({magnet: input, query: input})
        });
        const data = await res.json();
        if (data.success) {
          resBox.innerHTML = `
            <div class="space-y-2">
              <div class="font-bold text-white text-xs">📦 ${data.title}</div>
              <div class="text-[10px] text-zinc-400 font-mono">BTIH: ${data.hash || 'Auto-Detected'}</div>
              <div class="pt-2 flex gap-2">
                <button onclick="document.getElementById('magnet-input').value = '${data.magnet || ''}'; closeModal('modal-toolbox-collector'); openModal('modal-add-magnet');" class="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 text-white rounded-xl font-bold text-xs transition">
                  ⚡ Đẩy Sang Tải TorBox
                </button>
                <button onclick="setTab('agent'); sendQuickCommand('media-collector phân tích chi tiết census cho: ${data.title}'); closeModal('modal-toolbox-collector');" class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded-xl font-medium text-xs transition">
                  🤖 Hỏi AI Agent
                </button>
              </div>
            </div>
          `;
        } else {
          resBox.innerHTML = `<div class="text-red-400">❌ ${data.error}</div>`;
        }
      } catch (e) {
        if (resBox) resBox.innerHTML = `<div class="text-red-400">Lỗi kết nối: ${e}</div>`;
      }
    }

    async function searchTmdbLive() {
      const q = document.getElementById('tmdb-search-input')?.value?.trim();
      if (!q) {
        showToast('Vui lòng nhập tên phim cần tra cứu', 'warning');
        return;
      }
      const box = document.getElementById('tmdb-search-results');
      if (box) box.innerHTML = '<div class="p-6 text-center text-purple-400 animate-pulse text-xs">⏳ Đang kết nối TMDb API tra cứu siêu dữ liệu...</div>';
      
      try {
        const res = await fetch(`/api/tmdb/search?query=${encodeURIComponent(q)}`);
        const data = await res.json();
        const list = data.results || [];
        if (list.length === 0) {
          box.innerHTML = `<div class="p-6 text-center text-zinc-500 text-xs">${data.warning || data.error || 'Không tìm thấy kết quả phù hợp trên TMDb.'}</div>`;
          return;
        }

        let html = '';
        list.slice(0, 8).forEach(item => {
          const title = item.name || item.title || 'Unknown Title';
          const year = (item.first_air_date || item.release_date || '').slice(0, 4);
          const poster = item.poster_path ? `https://image.tmdb.org/t/p/w200${item.poster_path}` : 'https://placehold.co/200x300/18181b/71717a?text=No+Poster';
          const rating = (item.vote_average || 0).toFixed(1);
          const overview = item.overview || 'Không có mô tả nội dung tiếng Việt.';
          const mtype = item.media_type === 'tv' ? 'TV Series' : 'Movie';

          html += `
            <div class="p-3 rounded-2xl bg-zinc-950 border border-zinc-800 flex gap-3.5 items-start">
              <img src="${poster}" class="w-16 h-24 object-cover rounded-xl border border-zinc-800 shrink-0 shadow-md">
              <div class="flex-1 min-w-0 space-y-1">
                <div class="flex items-center justify-between gap-2">
                  <div class="font-bold text-white text-xs truncate">${title}</div>
                  <span class="px-1.5 py-0.2 rounded bg-purple-500/10 text-purple-400 border border-purple-500/30 text-[9px] font-bold shrink-0">⭐ ${rating}</span>
                </div>
                <div class="text-[10px] text-zinc-400 font-mono">${mtype} • Năm: ${year || 'N/A'} • TMDb ID: ${item.id}</div>
                <p class="text-[11px] text-zinc-400 line-clamp-2 leading-relaxed">${overview}</p>
                <div class="pt-1 flex gap-2">
                  <button onclick="document.getElementById('collector-input').value = '${title.replace(/'/g, "\\'")}'; closeModal('modal-toolbox-tmdb'); openModal('modal-toolbox-collector');" class="px-2.5 py-1 rounded-lg bg-blue-600/20 text-blue-400 hover:bg-blue-600 hover:text-white border border-blue-500/30 text-[10px] font-bold transition">
                    🔍 Tìm Nguồn Phim
                  </button>
                  <button onclick="setTab('agent'); sendQuickCommand('tmdb-lookup tạo file NFO chuẩn cho ${title.replace(/'/g, "\\'")} (TMDb: ${item.id})'); closeModal('modal-toolbox-tmdb');" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-[10px] font-medium transition">
                    📝 Tạo NFO
                  </button>
                </div>
              </div>
            </div>
          `;
        });
        box.innerHTML = html;
      } catch (e) {
        if (box) box.innerHTML = `<div class="p-4 text-center text-red-400 text-xs">Lỗi: ${e}</div>`;
      }
    }


    // ---- library index (SQLite) & metadata build ----
    async function loadLibraryStats() {
      try {
        const d = await (await fetch('/api/library/stats')).json();
        const bar = document.getElementById('library-index-bar');
        const el = document.getElementById('library-index-stats');
        if (!d.success || !el) return;
        const s = d.drive;
        const tb = (s.bytes / 1099511627776).toFixed(2);
        const age = s.last_refresh ? Math.round((Date.now()/1000 - s.last_refresh)/60) : null;
        el.textContent = `📚 ${s.shows} shows • ${s.files} file • ${tb} TB • poster ${s.with_poster}/${s.shows} • nfo ${s.with_nfo}/${s.shows}`
          + (age !== null ? ` • cập nhật ${age} phút trước` : '');
        if (bar) bar.classList.remove('hidden');
        window.__missingAssets = d.missing_assets || [];
      } catch (e) { /* index not ready yet */ }
    }

    async function refreshLibraryIndex() {
      showToast('⚡ Đang lập chỉ mục lại thư viện Google Drive...', 'info', 3000);
      try {
        const d = await (await fetch('/api/library/refresh', {method:'POST', headers:{'Content-Type':'application/json'}, body:'{}'})).json();
        showToast(d.message || 'Đã lập chỉ mục lại.', d.refreshed ? 'success' : 'info');
        await loadLibraryStats();
      } catch (e) { showToast(`❌ ${e}`, 'error'); }
    }

    async function buildLibraryMetadata() {
      const missing = window.__missingAssets || [];
      const n = missing.length;
      if (n === 0) {
        showToast('Mọi show đã có đủ poster / fanart / NFO.', 'success');
        return;
      }
      const preview = missing.slice(0, 8).map(m => `• ${m.name} (${m.missing.join(', ')})`).join('\n');
      if (!confirm(`Dựng metadata cho ${n} show còn thiếu?\n\n${preview}${n > 8 ? `\n… và ${n-8} show khác` : ''}\n\n` +
                   `Sẽ tải poster/fanart/NFO từ TMDb và GHI vào thư mục phim trên Google Drive.\n` +
                   `Show nào khớp TMDb không chắc chắn sẽ bị bỏ qua để chờ bạn duyệt.`)) return;
      try {
        const d = await (await fetch('/api/library/build', {
          method:'POST', headers:{'Content-Type':'application/json'},
          body: JSON.stringify({ targets:['drive'], only_missing:true })
        })).json();
        if (!d.success) { showToast(`❌ ${d.error}`, 'error'); return; }
        showToast(d.message, 'success');
        pollLibraryBuild();
      } catch (e) { showToast(`❌ ${e}`, 'error'); }
    }

    async function pollLibraryBuild() {
      const btn  = document.getElementById('btn-build-meta');
      const state = document.getElementById('library-build-state');
      const wrap = document.getElementById('library-build-barwrap');
      const bar  = document.getElementById('library-build-bar');
      const log  = document.getElementById('library-build-log');
      document.getElementById('library-index-bar')?.classList.remove('hidden');
      if (wrap) wrap.classList.remove('hidden');
      if (btn) btn.disabled = true;

      const icon = { built:'✅', skipped:'⏭️', needs_review:'⚠️', error:'❌', canceled:'🛑' };
      const tick = async () => {
        let st;
        try { st = await (await fetch('/api/library/build/status')).json(); }
        catch (e) { return finish(); }
        const pct = st.total ? Math.round(st.done / st.total * 100) : 0;
        if (bar) bar.style.width = `${pct}%`;
        if (state) state.textContent = st.error ? `❌ ${st.error}`
          : st.running ? `${st.done}/${st.total} — ${st.current}` : `Hoàn tất ${st.done}/${st.total}`;
        if (log) log.innerHTML = (st.results || []).slice(-12).reverse().map(r =>
          `<div class="truncate"><span>${icon[r.status] || '•'}</span> <span class="text-zinc-300">${r.folder}</span>` +
          `<span class="text-zinc-500"> — ${r.detail || r.status}</span></div>`).join('');
        if (st.running) { setTimeout(tick, 1200); } else { finish(); }
      };
      const finish = async () => {
        if (btn) btn.disabled = false;
        await refreshLibraryIndex();
        if (typeof fetchData === 'function') fetchData();
      };
      tick();
    }

    function triggerPlexScan() {
      // Clear localStorage cache and refresh current view
      seasonFilesMemoryCache = {};
      try { localStorage.removeItem('gdrive_season_cache_v1'); } catch (e) {}
      if (currentActiveShow && currentActiveShow.seasonList.length > 0) {
        const firstSeason = currentActiveShow.seasonList[0].split(" (")[0].trim();
        const showFolder = currentActiveShow.path.split('/')[2];
        loadSeasonEpisodes(showFolder, firstSeason, true);
      }
      showToast('⚡ Đã làm mới toàn bộ bộ nhớ Cache Google Drive!', 'success');
    }


export {
  scanNasLibraries,
  checkGDriveConnection,
  purgeStagingManual,
  inspectCollectorSource,
  searchTmdbLive,
  loadLibraryStats,
  refreshLibraryIndex,
  buildLibraryMetadata,
  pollLibraryBuild,
  triggerPlexScan
};
