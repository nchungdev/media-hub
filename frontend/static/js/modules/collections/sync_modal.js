/**
 * Sync Media Modal Controller & Subtitle Translation Actions
 * Hỗ trợ chọn danh sách video (nhiều chất lượng), phụ đề (internal/external) và chiều đồng bộ.
 */
import { showToast } from '../../core/toast.js';

let currentSyncContext = null;

/**
 * Mở popup đồng bộ cho một tập phim cụ thể trong bộ sưu tập
 * @param {string} colId ID bộ sưu tập
 * @param {string} epKey Mã tập phim (e.g. S01E01 hoặc tên video)
 */
export function openSyncMediaModal(colId, epKey) {
  const collections = window.allMediaCollections || [];
  const col = collections.find(c => c.id === colId);
  if (!col) {
    showToast('Không tìm thấy thông tin bộ phim', 'warning');
    return;
  }

  let targetEp = null;
  let targetSeason = null;
  for (const s of (col.seasons || [])) {
    const ep = (s.episodes || []).find(e => e.key === epKey);
    if (ep) {
      targetEp = ep;
      targetSeason = s;
      break;
    }
  }

  if (!targetEp) {
    showToast(`Không tìm thấy tập phim: ${epKey}`, 'warning');
    return;
  }

  currentSyncContext = {
    colId,
    colTitle: col.title,
    epKey,
    season: targetSeason,
    episode: targetEp
  };

  // Header
  const titleEl = document.getElementById('sync-media-show-title');
  const badgeEl = document.getElementById('sync-media-ep-badge');
  if (titleEl) titleEl.textContent = `${col.title} ${targetSeason ? `• ${targetSeason.name}` : ''}`;
  if (badgeEl) badgeEl.textContent = targetEp.key;

  // Render Video list
  const videoListEl = document.getElementById('sync-media-video-list');
  const videoCountEl = document.getElementById('sync-video-count');
  const videoFiles = targetEp.video_files && targetEp.video_files.length > 0 
    ? targetEp.video_files 
    : [{
        name: targetEp.name,
        path: targetEp.name,
        quality: '1080p',
        size_mb: 0.0
      }];

  if (videoCountEl) videoCountEl.textContent = `${videoFiles.length} bản video`;

  if (videoListEl) {
    videoListEl.innerHTML = videoFiles.map((vf, idx) => `
      <label class="flex items-center justify-between p-3 rounded-2xl bg-zinc-900/70 border border-zinc-800 hover:border-blue-500/40 cursor-pointer transition">
        <div class="flex items-center gap-3 min-w-0">
          <input type="checkbox" name="sync_video_item" value="${idx}" checked class="rounded border-zinc-700 bg-zinc-950 text-blue-600 focus:ring-0">
          <div class="min-w-0 space-y-0.5">
            <div class="text-xs font-semibold text-white truncate flex items-center gap-1.5" title="${vf.name}">
              <span>🎬</span>
              <span class="truncate">${vf.name}</span>
            </div>
            <div class="flex items-center gap-2 text-[10px] text-zinc-400 font-mono">
              <span class="px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300 font-bold border border-blue-500/30">${vf.quality}</span>
              ${vf.size_mb > 0 ? `<span>• ${vf.size_mb} MB</span>` : ''}
            </div>
          </div>
        </div>
        <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-zinc-950 border border-zinc-800 text-zinc-400 shrink-0">
          Local
        </span>
      </label>
    `).join('');
  }

  // Render Subtitle list
  const subListEl = document.getElementById('sync-media-sub-list');
  const subFiles = targetEp.subtitle_files || [];
  if (subListEl) {
    if (subFiles.length === 0) {
      subListEl.innerHTML = `
        <div class="p-4 text-center rounded-xl bg-zinc-900/40 border border-dashed border-zinc-800 text-zinc-500 text-xs">
          Tập này chưa có file phụ đề đi kèm.
        </div>
      `;
    } else {
      subListEl.innerHTML = subFiles.map((sf, idx) => {
        const isInternal = sf.is_internal;
        const isVi = sf.lang === 'vi';
        const langBadge = isVi ? 'VIETSUB' : sf.lang.toUpperCase();
        const typeBadge = isInternal 
          ? `<span class="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold bg-indigo-500/20 text-indigo-300 border border-indigo-500/40">INTERNAL</span>`
          : `<span class="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">EXTERNAL</span>`;

        return `
          <div class="flex items-center justify-between p-2.5 rounded-xl bg-zinc-900/50 border border-zinc-800/80 hover:border-zinc-700 transition">
            <label class="flex items-center gap-2.5 min-w-0 cursor-pointer flex-1 mr-2">
              <input type="checkbox" name="sync_sub_item" value="${idx}" checked class="rounded border-zinc-700 bg-zinc-950 text-blue-600 focus:ring-0">
              <div class="min-w-0 space-y-0.5">
                <div class="text-xs text-zinc-200 truncate flex items-center gap-1.5" title="${sf.name}">
                  <span>${isInternal ? '📼' : '📄'}</span>
                  <span class="truncate">${sf.name}</span>
                </div>
                <div class="flex items-center gap-1.5 text-[10px]">
                  ${typeBadge}
                  <span class="px-1.5 py-0.5 rounded text-[9px] font-bold ${isVi ? 'bg-purple-500/20 text-purple-300' : 'bg-zinc-800 text-zinc-400'}">${langBadge}</span>
                  <span class="font-mono text-zinc-500 uppercase">.${sf.format}</span>
                  ${sf.size_kb > 0 ? `<span class="font-mono text-zinc-500">• ${sf.size_kb} KB</span>` : ''}
                </div>
              </div>
            </label>
            <div class="shrink-0 flex items-center gap-1.5">
              <button onclick="translateSingleSubtitle('${col.title}', '${targetEp.key}', '${sf.name}', ${Boolean(isInternal)})" class="px-2 py-0.5 rounded bg-purple-600/20 hover:bg-purple-600 text-[10px] font-semibold text-purple-300 hover:text-white border border-purple-500/30 transition flex items-center gap-1" title="Dịch phụ đề này sang tiếng Việt">
                <span>🚀</span> Dịch
              </button>
              ${!isInternal ? `
                <a href="/api/subtitles/vtt?path=${encodeURIComponent(sf.path)}" target="_blank" class="px-2 py-0.5 rounded bg-zinc-950 hover:bg-zinc-800 text-[10px] text-zinc-400 hover:text-white border border-zinc-800 transition" title="Xem WebVTT">
                  👁️ VTT
                </a>
              ` : ''}
            </div>
          </div>
        `;
      }).join('');
    }
  }

  const selectAllSubs = document.getElementById('sync-select-all-subs');
  if (selectAllSubs) selectAllSubs.checked = true;

  if (typeof window.openModal === 'function') {
    window.openModal('modal-sync-media');
  }
}

/**
 * Check/uncheck tất cả các phụ đề trong modal
 */
export function toggleAllSyncSubs(checked) {
  const checkboxes = document.querySelectorAll('input[name="sync_sub_item"]');
  checkboxes.forEach(cb => cb.checked = checked);
}

/**
 * Kích hoạt đồng bộ media theo lựa chọn của người dùng
 */
export async function startMediaSync() {
  if (!currentSyncContext) {
    showToast('Chưa chọn nội dung đồng bộ', 'warning');
    return;
  }

  const { colTitle, epKey, episode } = currentSyncContext;
  const btn = document.getElementById('btn-start-sync-media');

  // Lấy các mục đã chọn
  const selectedVideoIdxs = Array.from(document.querySelectorAll('input[name="sync_video_item"]:checked')).map(cb => parseInt(cb.value));
  const selectedSubIdxs = Array.from(document.querySelectorAll('input[name="sync_sub_item"]:checked')).map(cb => parseInt(cb.value));
  const directionEl = document.querySelector('input[name="sync-direction"]:checked');
  const direction = directionEl ? directionEl.value : 'local_to_nas';
  const autoPurge = document.getElementById('sync-opt-autopurge')?.checked || false;

  if (selectedVideoIdxs.length === 0 && selectedSubIdxs.length === 0) {
    showToast('Vui lòng chọn ít nhất 1 video hoặc phụ đề để đồng bộ', 'warning');
    return;
  }

  if (btn) {
    btn.disabled = true;
    btn.innerHTML = `<span>⏳</span> Đang Đồng Bộ...`;
  }

  try {
    const res = await fetch('/api/subtitles/sync', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        title: colTitle,
        ep_key: epKey,
        direction,
        auto_purge: autoPurge,
        video_count: selectedVideoIdxs.length,
        sub_count: selectedSubIdxs.length
      })
    });

    const data = await res.json();
    if (data.success) {
      showToast(`🚀 Đồng bộ thành công: ${data.message || 'Dữ liệu đã được chuyển'}`, 'success', 3500);
      if (typeof window.closeModal === 'function') {
        window.closeModal('modal-sync-media');
      }
      if (typeof window.loadMediaCollections === 'function') {
        window.loadMediaCollections(true);
      }
    } else {
      showToast(`Lỗi đồng bộ: ${data.error || 'Thao tác không thành công'}`, 'error', 4000);
    }
  } catch (err) {
    showToast(`Lỗi kết nối: ${err.message}`, 'error', 4000);
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.innerHTML = `<span>🚀</span> Bắt Đầu Đồng Bộ`;
    }
  }
}

/**
 * Kích hoạt dịch trực tiếp cho 1 file hoặc track phụ đề cụ thể
 * @param {string} showTitle Tên phim/series
 * @param {string} epKey Mã tập
 * @param {string} subName Tên file sub hoặc tên track phụ đề
 * @param {boolean} isInternal True nếu nhúng trong container video
 */
export function translateSingleSubtitle(showTitle, epKey, subName, isInternal) {
  if (window.quotaGuardLocked) {
    const q = window.quotaGuardStatus;
    const msg = q ? (q.day.used >= q.day.limit ? `Hôm nay đã dịch chạm trần ${q.day.used}/${q.day.limit} tập.` : `Tuần này đã dịch chạm trần.`) : 'Translation Quota Guard đã tạm khóa.';
    showToast(`🛑 ${msg}`, 'warning', 4000);
    return;
  }

  const typeDesc = isInternal ? `từ track nhúng trong container video (${subName})` : `từ file phụ đề nguồn rời (${subName})`;
  const mediaId = 'media-show-' + (showTitle || '').toLowerCase().replace(/[^a-z0-9]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
  const command = `translate-subtitle: DỊCH PHỤ ĐỀ CHO TẬP ${epKey} của "${showTitle}".
Nguồn phụ đề: ${typeDesc}.
Yêu cầu: Dịch sang tiếng Việt chuyên sâu, giữ nguyên timing, xuất bản ra 3 định dạng (.vi.ass, .vi.srt, .vi.vtt).
BẮT BUỘC in log console theo 5 bước: [BƯỚC 1/5: Khảo sát nguồn], [BƯỚC 2/5: Glossary], [BƯỚC 3/5: Dịch thuật], [BƯỚC 4/5: Xuất bản], [BƯỚC 5/5: Audit] để hiển thị lên Live Console.`;

  if (typeof window.sendQuickCommand === 'function') {
    window.sendQuickCommand(command, mediaId);
  }
  showToast(`🚀 Đã gửi yêu cầu dịch phụ đề tập ${epKey} sang AI Agent!`, 'success', 3000);
}

// Mount to window
Object.assign(window, {
  openSyncMediaModal,
  toggleAllSyncSubs,
  startMediaSync,
  translateSingleSubtitle
});
