/**
 * Services Module — theo doi cac worker nen (indexer, watcher, sync worker).
 * Nguon du lieu: GET /api/services/workers
 */
import { apiFetch } from '../core/api.js';

let servicesTimer = null;

const WORKER_META = {
  'indexer/local':      { icon: '💻', label: 'Index thư viện Local',    desc: 'Quét .media-hub/_franchise mỗi 60 giây' },
  'indexer/nas':        { icon: '🖥️', label: 'Index thư viện NAS',      desc: 'Liệt kê qua SSH mỗi 10 phút' },
  'indexer/jellyfin':   { icon: '🎬', label: 'Index Jellyfin',          desc: 'Đọc jellyfin.db, chỉ tải khi DB đổi' },
  'indexer/gdrive-nfo': { icon: '☁️', label: 'Index Google Drive',      desc: 'Đọc <uniqueid> trong .nfo mỗi 15 phút' },
  'sync_worker':        { icon: '⬇️', label: 'Hàng đợi tải xuống',      desc: 'Điều phối aria2 RPC mỗi 3 giây' },
  'watcher/_franchise': { icon: '👁️', label: 'Theo dõi thay đổi file',  desc: 'Tự làm mới khi thư mục đổi' },
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
      box.innerHTML = `<div class="p-8 text-center text-zinc-500 text-sm">
        Chưa có worker nào báo cáo. Chúng đăng ký sau vòng chạy đầu tiên.
      </div>`;
      return;
    }

    box.innerHTML = workers.map((w) => {
      const meta = WORKER_META[w.name] || { icon: '⚙️', label: w.name, desc: '' };
      const st = STATE_STYLE[w.state] || STATE_STYLE.idle;
      // items = -1 la quy uoc "khong co gi de lam", vd DB khong doi nen bo qua.
      const items = w.items === -1 ? '—' : w.items;
      const errBadge = w.errors > 0
        ? `<span class="text-[10px] px-2 py-0.5 rounded-full bg-red-500/10 text-red-400 border border-red-500/20 font-mono">${w.errors} lỗi</span>`
        : '';

      return `
      <div class="px-4 sm:px-5 py-3.5 border-b border-zinc-800/60 last:border-0 hover:bg-zinc-900/40 transition">
        <div class="flex items-start justify-between gap-4 flex-wrap">
          <div class="flex items-start gap-3 min-w-0">
            <div class="w-9 h-9 rounded-xl bg-zinc-900 border border-zinc-800 flex items-center justify-center text-base shrink-0">
              ${meta.icon}
            </div>
            <div class="min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="font-bold text-sm text-white">${esc(meta.label)}</span>
                <span class="inline-flex items-center gap-1.5 text-[10px] px-2 py-0.5 rounded-full border font-bold ${st.cls}">
                  <span class="w-1.5 h-1.5 rounded-full ${st.dot}"></span>${st.text}
                </span>
                ${errBadge}
              </div>
              <div class="text-xs text-zinc-500 mt-0.5">${esc(meta.desc)}</div>
              ${w.message ? `<div class="text-xs text-zinc-400 mt-1 font-mono truncate">${esc(w.message)}</div>` : ''}
            </div>
          </div>
          <div class="flex items-center gap-5 text-right shrink-0">
            <div>
              <div class="text-sm font-bold text-white font-mono">${items}</div>
              <div class="text-[10px] text-zinc-500 uppercase tracking-wide">mục</div>
            </div>
            <div>
              <div class="text-sm font-bold text-zinc-300 font-mono">${w.runs ?? 0}</div>
              <div class="text-[10px] text-zinc-500 uppercase tracking-wide">lượt chạy</div>
            </div>
            <div class="hidden sm:block min-w-[92px]">
              <div class="text-xs font-semibold text-zinc-400">${timeAgo(w.last_finish)}</div>
              <div class="text-[10px] text-zinc-500 uppercase tracking-wide">lần cuối</div>
            </div>
          </div>
        </div>
      </div>`;
    }).join('');
  } catch (e) {
    box.innerHTML = `<div class="p-8 text-center text-red-400 text-sm">
      Không tải được trạng thái worker: ${esc(e?.message || e)}
    </div>`;
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

Object.assign(window, {
  loadServicesStatus,
  startServicesAutoRefresh,
  stopServicesAutoRefresh,
});
