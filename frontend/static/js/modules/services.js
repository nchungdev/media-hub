/**
 * Services Module — theo doi cac worker nen (indexer, watcher, sync worker).
 * Nguon du lieu: GET /api/services/workers
 */
import { apiFetch } from '../core/api.js';

let servicesTimer = null;

const WORKER_META = {
  'indexer/local':      { icon: '📝', label: 'Index kho Draft',         desc: 'Chạy theo sự kiện từ watcher, lưới an toàn 15 phút' },
  'indexer/jellyfin':   { icon: '🎬', label: 'Index Jellyfin',          desc: 'Đọc jellyfin.db, chỉ tải khi DB đổi' },
  'indexer/plex':       { icon: '🍿', label: 'Index Plex',              desc: 'Đọc thư viện Plex, chỉ tải khi DB đổi' },
  'indexer/gdrive-nfo': { icon: '☁️', label: 'Index Google Drive',      desc: 'Đọc <uniqueid> trong .nfo, chỉ tải khi đổi' },
  'sync_worker':        { icon: '⬇️', label: 'Hàng đợi tải xuống',      desc: 'Điều phối aria2 RPC — 2 giây khi bận, 30 giây khi rỗng' },
  'watcher/_franchise': { icon: '👁️', label: 'Theo dõi thay đổi file',  desc: 'Tự làm mới khi thư mục _franchise đổi' },
  'agy_daemon':         { icon: '🤖', label: 'Daemon agy',              desc: 'Tiến trình agy thường trú, giao tiếp qua stream-json' },
  'agent_queue':        { icon: '📨', label: 'Hàng đợi lệnh AI',        desc: 'Đẩy lệnh chờ vào daemon agy' },
};

const STATE_STYLE = {
  ok:      { cls: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20', dot: 'bg-emerald-400', text: 'Bình thường' },
  running: { cls: 'bg-blue-500/10 text-blue-400 border-blue-500/20',          dot: 'bg-blue-400 animate-pulse', text: 'Đang chạy' },
  error:   { cls: 'bg-red-500/10 text-red-400 border-red-500/20',             dot: 'bg-red-400', text: 'Lỗi' },
  idle:    { cls: 'bg-zinc-700/20 text-zinc-400 border-zinc-600/30',          dot: 'bg-zinc-500', text: 'Chờ' },
};

function timeAgo(ts) {
  if (!ts) return '—';
  const s = Math.max(0, Math.floor(Date.now() / 1000 - ts));
  if (s < 60) return `${s} giây trước`;
  if (s < 3600) return `${Math.floor(s / 60)} phút trước`;
  if (s < 86400) return `${Math.floor(s / 3600)} giờ trước`;
  return `${Math.floor(s / 86400)} ngày trước`;
}

function esc(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

export async function loadServicesStatus() {
  const box = document.getElementById('services-list');
  if (!box) return;

  try {
    const data = await apiFetch('/api/services/workers');
    const workers = data?.workers || [];

    document.getElementById('services-total').textContent = data.total ?? 0;
    document.getElementById('services-running').textContent = data.running ?? 0;
    document.getElementById('services-errors').textContent = data.errors ?? 0;

    if (!workers.length) {
      box.innerHTML = `<tr><td colspan="6" class="p-8 text-center text-zinc-500 text-sm">
        Chưa có worker nào báo cáo. Chúng đăng ký sau vòng chạy đầu tiên.
      </td></tr>`;
      return;
    }

    box.innerHTML = workers.map((w) => {
      const meta = WORKER_META[w.name] || { icon: '⚙️', label: w.name, desc: '' };
      const isStopped = w.enabled === false || w.state === 'stopped';
      const st = isStopped 
        ? { cls: 'bg-zinc-800 text-zinc-400 border-zinc-700 shadow-sm', dot: 'bg-zinc-500', text: 'Đã dừng' }
        : (STATE_STYLE[w.state] || STATE_STYLE.idle);

      // items = -1 la quy uoc "khong co gi de lam", vd DB khong doi nen bo qua.
      const items = w.items === -1 ? '—' : w.items;
      const errBadge = w.errors > 0
        ? `<span class="text-[10px] px-2 py-0.5 rounded-full bg-red-500/15 text-red-400 border border-red-500/30 font-mono font-bold">${w.errors} lỗi</span>`
        : '';

      return `
      <tr class="hover:bg-zinc-900/60 transition">
        <!-- Cột 1: Dịch vụ -->
        <td class="px-4 py-3.5 min-w-[220px] sm:min-w-[260px]">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-base shrink-0 shadow-inner">
              ${meta.icon}
            </div>
            <div class="min-w-0">
              <div class="font-bold text-sm text-white flex items-center gap-1.5 truncate">
                <span>${esc(meta.label)}</span>
                <span class="text-[10px] font-mono text-zinc-500">(${esc(w.name)})</span>
              </div>
              <div class="text-xs text-zinc-400 mt-0.5">${esc(meta.desc)}</div>
              ${w.message ? `<div class="text-[11px] text-zinc-500 mt-0.5 font-mono truncate" title="${esc(w.message)}">${esc(w.message)}</div>` : ''}
            </div>
          </div>
        </td>

        <!-- Cột 2: Trạng Thái (CỘT RIÊNG) -->
        <td class="px-3 py-3.5 text-center w-36">
          <div class="inline-flex items-center justify-center flex-wrap gap-1">
            <span class="inline-flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full border font-bold ${st.cls}">
              <span class="w-2 h-2 rounded-full ${st.dot}"></span>
              ${st.text}
            </span>
            ${errBadge}
          </div>
        </td>

        <!-- Cột 3: Mục -->
        <td class="px-3 py-3.5 text-center w-24">
          <span class="font-mono text-sm font-bold text-white">${items}</span>
        </td>

        <!-- Cột 4: Lượt Chạy -->
        <td class="px-3 py-3.5 text-center w-24">
          <span class="font-mono text-sm font-bold text-zinc-300">${w.runs ?? 0}</span>
        </td>

        <!-- Cột 5: Lần Cuối -->
        <td class="px-3 py-3.5 text-center w-32">
          <span class="text-xs font-semibold text-zinc-400 font-mono">${timeAgo(w.last_finish)}</span>
        </td>

        <!-- Cột 6: Thao Tác (Start / Restart / Stop) -->
        <td class="px-4 py-3.5 text-right min-w-[190px]">
          <div class="flex items-center justify-end gap-1.5">
            ${isStopped
              ? `<button onclick="controlWorker('${esc(w.name)}','start')" class="px-2.5 py-1 rounded-xl text-xs font-bold bg-emerald-600/20 hover:bg-emerald-600 text-emerald-400 hover:text-white border border-emerald-500/30 transition flex items-center gap-1 shadow-sm" title="Khởi động / Bật service này">
                  <span>▶</span> Start
                 </button>`
              : `<button onclick="controlWorker('${esc(w.name)}','stop')" class="px-2.5 py-1 rounded-xl text-xs font-bold bg-zinc-900 hover:bg-red-600 text-zinc-300 hover:text-white border border-zinc-700 hover:border-red-500/30 transition flex items-center gap-1 shadow-sm" title="Tạm dừng service này">
                  <span>⏸</span> Stop
                 </button>`}
            <button onclick="controlWorker('${esc(w.name)}','restart')" class="px-2.5 py-1 rounded-xl text-xs font-bold bg-violet-600/20 hover:bg-violet-600 text-violet-400 hover:text-white border border-violet-500/30 transition flex items-center gap-1 shadow-sm" title="Chạy lại chu kỳ ngay lập tức">
              <span>↻</span> Restart
            </button>
          </div>
        </td>
      </tr>`;
    }).join('');
  } catch (e) {
    box.innerHTML = `<tr><td colspan="6" class="p-8 text-center text-red-400 text-sm">
      Không tải được trạng thái worker: ${esc(e?.message || e)}
    </td></tr>`;
  }
}

/** Bat/tat tu lam moi khi vao/roi tab, tranh goi API vo ich luc dang xem tab khac. */
export function startServicesAutoRefresh() {
  loadServicesStatus();
  if (servicesTimer) clearInterval(servicesTimer);
  servicesTimer = setInterval(loadServicesStatus, 5000);
}

export function stopServicesAutoRefresh() {
  if (servicesTimer) {
    clearInterval(servicesTimer);
    servicesTimer = null;
  }
}

/**
 * Bật / dừng / chạy lại ngay một worker nền.
 * Dừng không làm thread thoát — chỉ hạ cờ enabled, nên bật lại được ngay
 * mà không phải khởi động lại app.
 */
export async function controlWorker(name, action) {
  try {
    const res = await fetch(
      `/api/services/workers/${encodeURIComponent(name)}/${action}`,
      { method: 'POST' }
    );
    const d = await res.json();
    if (typeof window.showToast === 'function') {
      window.showToast(
        d.success ? `${name}: ${d.message}` : `Lỗi: ${d.message || 'không rõ'}`,
        d.success ? 'success' : 'error'
      );
    }
  } catch (e) {
    console.error('controlWorker error:', e);
  }
  // Đọc lại ngay để nút đổi trạng thái, không chờ vòng làm mới 5 giây.
  loadServicesStatus();
}

Object.assign(window, {
  loadServicesStatus,
  startServicesAutoRefresh,
  stopServicesAutoRefresh,
  controlWorker,
});
