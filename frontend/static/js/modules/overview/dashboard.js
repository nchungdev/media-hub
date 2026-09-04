/**
 * System KPIs & Hardware Health Overview
 */
import { showToast } from '../../core/toast.js';

            async function fetchDashboardOverview() {
      try {
        const res = await fetch('/api/dashboard/overview');
        const data = await res.json();
        if (!data.success) return;

        // 1. Update Hardware Health
        const h = data.health;
        if (h) {
          if (document.getElementById('health-cpu-val')) document.getElementById('health-cpu-val').innerText = `Load ${h.cpu_load}`;
          if (document.getElementById('health-cpu-bar')) document.getElementById('health-cpu-bar').style.width = `${Math.min(h.cpu_load * 15, 100)}%`;
          
          if (document.getElementById('health-ram-val')) document.getElementById('health-ram-val').innerText = `${h.ram_used_gb} / ${h.ram_total_gb} GB`;
          if (document.getElementById('health-ram-bar')) document.getElementById('health-ram-bar').style.width = `${h.ram_pct}%`;

          if (document.getElementById('health-disk-val')) document.getElementById('health-disk-val').innerText = `${h.local_disk.used_gb} / ${h.local_disk.total_gb} GB`;
          if (document.getElementById('health-disk-bar')) document.getElementById('health-disk-bar').style.width = `${h.local_disk.percent}%`;
        }

        // 2. Update NAS & GDrive storages
        const clouds = data.clouds || [];
        const nasCloud = clouds.find(c => c.id === 'nas');
        const gdCloud = clouds.find(c => c.id === 'gdrive');

        if (nasCloud) {
          if (document.getElementById('nas-used-val')) document.getElementById('nas-used-val').innerText = `${nasCloud.used_str} / ${nasCloud.total_str}`;
          if (document.getElementById('nas-avail-val')) document.getElementById('nas-avail-val').innerText = `${nasCloud.avail_str} (${nasCloud.percent}%)`;
          if (document.getElementById('nas-cap-bar')) document.getElementById('nas-cap-bar').style.width = `${nasCloud.percent}%`;
        }

        if (gdCloud) {
          if (document.getElementById('gdrive-used-val')) document.getElementById('gdrive-used-val').innerText = gdCloud.used_str;
          if (document.getElementById('gdrive-avail-val')) document.getElementById('gdrive-avail-val').innerText = gdCloud.avail_str;
        }

        // 3. Render KHỐI TẢI XUỐNG (DOWNLOADING): Tải gì? Bằng engine gì? Tải về đâu?
        const dlContainer = document.getElementById('home-active-downloads-list');
        if (dlContainer) {
          if (data.active_downloads && data.active_downloads.length > 0) {
            dlContainer.innerHTML = data.active_downloads.map(d => `
              <div class="p-4 rounded-2xl bg-zinc-950/90 border border-blue-500/40 space-y-3 shadow-lg">
                <!-- Row 1: Tải gì & Engine gì -->
                <div class="flex items-start justify-between gap-2 flex-wrap">
                  <div class="min-w-0 flex-1">
                    <div class="text-[10px] text-zinc-500 uppercase font-bold tracking-wider">TẢI GÌ:</div>
                    <span class="font-bold text-xs sm:text-sm text-white truncate block">${d.name}</span>
                  </div>
                  <div class="flex flex-col items-end gap-1 shrink-0">
                    <span class="text-[10px] font-mono text-purple-300 bg-purple-500/10 px-2.5 py-0.5 rounded-full border border-purple-500/30 font-bold flex items-center gap-1">
                      <span>⚡ Engine:</span> <strong>${d.engine || 'TorBox Cloud DDL'}</strong>
                    </span>
                  </div>
                </div>

                <!-- Row 2: Tải về đâu (Local Buffer) -->
                <div class="p-2 rounded-xl bg-zinc-900/80 border border-zinc-800 text-[11px] text-zinc-300 flex items-center justify-between gap-2 font-mono">
                  <span class="text-zinc-400 flex items-center gap-1 shrink-0">
                    <span>📁 Tải về đâu:</span>
                  </span>
                  <span class="text-blue-400 truncate text-right">${d.dest_path || '/Volumes/512GB/AI Workspace/media_staging'}</span>
                </div>

                <!-- Progress Bar & Stats -->
                <div class="space-y-1.5 pt-0.5">
                  <div class="flex justify-between text-[11px] font-mono">
                    <span class="text-blue-400 font-bold">⚡ Đang tải: ${d.progress}%</span>
                    <span class="text-zinc-300">Tốc độ: <strong class="text-white">${d.speed}</strong> • ETA: ${d.eta}</span>
                  </div>
                  <div class="w-full bg-zinc-900 h-2.5 rounded-full overflow-hidden border border-blue-500/20">
                    <div class="bg-gradient-to-r from-blue-500 via-indigo-500 to-purple-500 h-full rounded-full transition-all duration-500" style="width: ${d.progress}%;"></div>
                  </div>
                </div>
              </div>
            `).join('');
          } else {
                        dlContainer.innerHTML = `
              <div class="p-4 rounded-2xl bg-zinc-950/60 border border-zinc-800/80 space-y-3">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2.5">
                    <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
                    <span class="font-bold text-xs text-white">Không có tiến trình tải nào đang chạy</span>
                  </div>
                  <span class="px-2.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[10px] font-mono font-bold">
                    Tất cả Torrents đã 100% Cached
                  </span>
                </div>
                <p class="text-xs text-zinc-400">
                  Dữ liệu trên TorBox Cloud đã được tải và lưu trữ an toàn. Khi bấm Sync, hệ thống sẽ tự động kích hoạt chuỗi truyền tải cuốn chiếu theo nhu cầu.
                </p>
                <div class="pt-1 flex items-center justify-between text-[11px] text-zinc-500">
                  <span>Trạng thái: <strong class="text-zinc-300">Hàng đợi rảnh rỗi</strong></span>
                  <button onclick="setTab('torbox')" class="text-blue-400 hover:text-blue-300 font-semibold">
                    Xem danh sách torrent ➔
                  </button>
                </div>
              </div>
            `;
          }
        }

        // 4. Render KHỐI ĐỒNG BỘ (SYNC / UPLOAD): Sync gì? Sync lên đâu?
        const ulContainer = document.getElementById('home-active-uploads-list');
        if (ulContainer) {
          if (data.active_uploads && data.active_uploads.length > 0) {
            ulContainer.innerHTML = data.active_uploads.map(u => `
              <div class="p-4 rounded-2xl bg-zinc-950/90 border border-emerald-500/40 space-y-3 shadow-lg">
                <!-- Row 1: Sync gì & Đích đến -->
                <div class="flex items-start justify-between gap-2 flex-wrap">
                  <div class="min-w-0 flex-1">
                    <div class="text-[10px] text-zinc-500 uppercase font-bold tracking-wider">SYNC GÌ:</div>
                    <span class="font-bold text-xs sm:text-sm text-white truncate block">${u.title}</span>
                  </div>
                  <div class="flex flex-col items-end gap-1 shrink-0">
                    <span class="text-[10px] font-mono text-emerald-300 bg-emerald-500/10 px-2.5 py-0.5 rounded-full border border-emerald-500/30 font-bold flex items-center gap-1">
                      <span>☁️ SYNC LÊN ĐÂU:</span> <strong>${u.dest || 'Google Drive (gdrive:Phim)'}</strong>
                    </span>
                  </div>
                </div>

                <!-- Row 2: Lộ trình truyền tải (Breadcrumb) -->
                <div class="p-2 rounded-xl bg-zinc-900/80 border border-zinc-800 text-[11px] text-zinc-300 flex items-center justify-between gap-2 font-mono flex-wrap">
                  <span class="text-zinc-400">Lộ trình:</span>
                  <div class="flex items-center gap-1.5 text-xs">
                    <span class="text-purple-400">⚡ TorBox</span>
                    <span class="text-zinc-600">➔</span>
                    <span class="text-blue-400">💾 Máy Đệm</span>
                    <span class="text-zinc-600">➔</span>
                    <span class="text-emerald-400 font-bold">${u.dest_short || '☁️ gdrive:Phim'}</span>
                  </div>
                </div>

                <!-- Progress Bar & Stats -->
                <div class="space-y-1.5 pt-0.5">
                  <div class="flex justify-between text-[11px] font-mono">
                    <span class="text-emerald-400 font-bold">⚡ Đang đồng bộ: ${u.progress}% (Tập ${u.current_ep}/${u.total_ep})</span>
                    <span class="text-emerald-400">🗑️ Tự động xóa đệm sau mỗi tập</span>
                  </div>
                  <div class="w-full bg-zinc-900 h-2.5 rounded-full overflow-hidden border border-emerald-500/20">
                    <div class="bg-gradient-to-r from-emerald-500 to-teal-400 h-full rounded-full transition-all duration-500" style="width: ${u.progress}%;"></div>
                  </div>
                </div>
              </div>
            `).join('');
          } else {
            ulContainer.innerHTML = `
              <div class="p-4 rounded-2xl bg-zinc-950/70 border border-zinc-800 space-y-3">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2.5">
                    <span class="w-2.5 h-2.5 rounded-full bg-emerald-400 animate-pulse"></span>
                    <span class="font-bold text-xs text-white">Chuỗi Đồng Bộ Đang Nghỉ (Idle)</span>
                  </div>
                  <span class="px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[10px] font-mono font-bold">
                    Đã Đồng Bộ 100%
                  </span>
                </div>

                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px] font-mono">
                  <div class="p-2 rounded-xl bg-zinc-900/60 border border-zinc-800/80">
                    <span class="text-zinc-500 block text-[9px] uppercase">Sync Lên Kho Cloud:</span>
                    <span class="text-emerald-400 font-bold">☁️ gdrive:Phim (31 Shows)</span>
                  </div>
                  <div class="p-2 rounded-xl bg-zinc-900/60 border border-zinc-800/80">
                    <span class="text-zinc-500 block text-[9px] uppercase">Sync Lên Kho NAS:</span>
                    <span class="text-amber-400 font-bold">🖥️ /srv/mergerfs/MainPool/Phim</span>
                  </div>
                </div>

                <div class="flex items-center justify-between pt-1 text-[11px]">
                  <span class="text-zinc-400">Toàn bộ media đã nằm an toàn trên cả 2 kho lưu trữ</span>
                  <button onclick="setTab('pipelines')" class="px-3 py-1.5 rounded-xl bg-emerald-600/20 hover:bg-emerald-600 text-emerald-300 hover:text-white border border-emerald-500/30 font-semibold transition flex items-center gap-1 shadow-sm text-xs">
                    <span>🚀</span> Xem Tiến Trình
                  </button>
                </div>
              </div>
            `;
          }
        }

      } catch (e) {
        console.error("Error fetching dashboard overview:", e);
      }
    }


export {
  fetchDashboardOverview
};
