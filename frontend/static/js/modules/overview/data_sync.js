/**
 * Multi-source Background Data Sync (Pipelines, Torbox & Sidebar Badges)
 */
import { showToast } from '../../core/toast.js';

    async function fetchData() {
              // 1. Fetch Pipelines with Dynamic Active Sync Integration
        try {
          const res = await fetch('/api/pipelines');
          const data = await res.json();
          
          let dynamicTasks = [];

          // 1.1 Convert Active Syncs into top-priority Active Pipeline Tasks
          if (data.active_syncs && data.active_syncs.length > 0) {
            data.active_syncs.forEach((j, i) => {
              const targets = j.targets || [j.target || 'drive'];
              const targetLabel = targets.includes('gdrive') && targets.includes('nas') ? 'Google Drive & NAS Storage' : (targets.includes('nas') ? 'NAS Storage' : 'Google Drive');
              const prog = j.progress || 45.0;
              
              dynamicTasks.push({
                index: i + 1,
                status: j.status === 'done' ? 'done' : 'active',
                type: '⚡ Đang Đồng Bộ',
                title: j.name || `Torrent #${j.torrent_id}`,
                format: 'Direct Stream',
                destination: targetLabel,
                size: 'Đang truyền tải...',
                stage: j.status === 'done' ? '✓ Đã hoàn tất 100%' : `⚡ Đang xử lý truyền tải sang ${targetLabel}`,
                subInfo: `${prog}% hoàn tất • Đang chạy`,
                percent: prog,
                dl_percent: 100,
                dl_status: '✓ Đã có cache sẵn sàng từ TorBox Cloud',
                ul_percent: prog,
                ul_status: `Đang upload lên ${targetLabel}`
              });
            });
          }

          // 1.2 Default Master Catalog Tasks
          const catalogTasks = [
            {
              index: dynamicTasks.length + 1,
              status: 'done',
              type: '🎭 Anime BDRip',
              title: 'Monster (2004) [1080p BluRay DUAL FLAC]',
              format: '1080p BluRay Remix',
              destination: 'TV Shows/Monster (2004) {tvdb-74599}',
              size: '~150 GB (74 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '74 / 74 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 74/74 tập đã tải xong từ TorBox',
              ul_percent: 100,
              ul_status: '✓ 74/74 tập đã upload Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 2,
              status: 'done',
              type: '⚡ 3D Anime',
              title: 'WUKONG (Ngộ Không 2025)',
              format: '1080p WEB-DL',
              destination: 'TV Shows/The Westward Universe',
              size: '4.8 GB (12 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '12 / 12 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 12/12 tập đã tải xong từ TorBox',
              ul_percent: 100,
              ul_status: '✓ 12/12 tập đã upload Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 3,
              status: 'done',
              type: '🤖 Mecha Anime',
              title: 'Cross Fight B-Daman (2011 - Season 01)',
              format: '1080p WEB-DL',
              destination: 'TV Shows/Cross Fight B-Daman',
              size: '17.2 GB (51 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '51 / 51 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 51/51 tập đã tải xong từ TorBox',
              ul_percent: 100,
              ul_status: '✓ 51/51 tập đã upload Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 4,
              status: 'done',
              type: '🤖 Mecha Anime',
              title: 'Cross Fight B-Daman eS (Season 02)',
              format: '720p HDTV',
              destination: 'TV Shows/Cross Fight B-Daman eS',
              size: '11.93 GB (52 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '52 / 52 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 52/52 tập đã tải xong từ TorBox',
              ul_percent: 100,
              ul_status: '✓ 52/52 tập đã upload Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 5,
              status: 'done',
              type: '🚗 Mecha Anime',
              title: 'Transformers: Car Robots (2000)',
              format: '480p DVD Remaster',
              destination: 'TV Shows/Transformers: Car Robots (2000)',
              size: '10.5 GB (39 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '39 / 39 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 39/39 tập đã nạp từ TorBox Cloud',
              ul_percent: 100,
              ul_status: '✓ 39/39 tập đã có trên Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 6,
              status: 'done',
              type: '⚡ 3D Anime',
              title: 'Tây Hành Kỷ - Season 02',
              format: '1080p WEB-DL',
              destination: 'TV Shows/The Westward Universe',
              size: '12.36 GB (16 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '16 / 16 tập • Sẵn sàng',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 16/16 tập đã nạp từ TorBox Cloud',
              ul_percent: 100,
              ul_status: '✓ 16/16 tập đã có trên Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 7,
              status: 'done',
              type: '🤖 Mecha Anime',
              title: 'Cap Kakumei Bottleman DX',
              format: '1080p HDTV',
              destination: 'TV Shows/Cap Kakumei Bottleman DX',
              size: '17.09 GB (51 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '51 / 51 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 51/51 tập đã nạp từ TorBox Cloud',
              ul_percent: 100,
              ul_status: '✓ 51/51 tập đã có trên Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 8,
              status: 'done',
              type: '🎯 Classic Anime',
              title: 'B-Legend! Battle B-Daman (DVD)(mq)',
              format: '480p DVD',
              destination: 'TV Shows/B-Legend! Battle B-Daman',
              size: '13.62 GB (52 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '52 / 52 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 52/52 tập đã nạp từ TorBox Cloud',
              ul_percent: 100,
              ul_status: '✓ 52/52 tập đã có trên Drive • Cache Cleaned'
            },
            {
              index: dynamicTasks.length + 9,
              status: 'done',
              type: '🎯 Classic Anime',
              title: 'B-Legend! Battle B-Daman Fire Spirits!',
              format: '480p WEB-DL',
              destination: 'TV Shows/Battle B-Daman Fire Spirits',
              size: '20.30 GB (52 tập)',
              stage: '✓ Đã đồng bộ hoàn tất 100%',
              subInfo: '52 / 52 tập • Trọn bộ Complete',
              percent: 100,
              dl_percent: 100,
              dl_status: '✓ 52/52 tập đã nạp từ TorBox Cloud',
              ul_percent: 100,
              ul_status: '✓ 52/52 tập đã có trên Drive • Cache Cleaned'
            }
          ];

          // 1.3 Combine and Sort: ACTIVE tasks ALWAYS on TOP!
          let allTasks = [...dynamicTasks, ...catalogTasks];
          allTasks.sort((a, b) => {
            if (a.status === 'active' && b.status !== 'active') return -1;
            if (a.status !== 'active' && b.status === 'active') return 1;
            return a.index - b.index;
          });
          
          // Re-index cleanly
          allTasks.forEach((t, i) => t.index = i + 1);

          // Update counts on filter buttons
          const activeCount = allTasks.filter(t => t.status === 'active').length;
          const doneCount = allTasks.filter(t => t.status === 'done').length;
          const queuedCount = allTasks.filter(t => t.status === 'queued').length;

          const btnAll = document.getElementById('pipefilter-all');
          if (btnAll) btnAll.innerText = `Tất Cả (${allTasks.length})`;
          const btnActive = document.getElementById('pipefilter-active');
          if (btnActive) btnActive.innerText = `⚡ Đang Chạy (${activeCount})`;
          const btnDone = document.getElementById('pipefilter-done');
          if (btnDone) btnDone.innerText = `✓ Hoàn Thành (${doneCount})`;
          const btnQueued = document.getElementById('pipefilter-queued');
          if (btnQueued) btnQueued.innerText = `⏳ Hàng Đợi (${queuedCount})`;

          // Filter by status
          const filteredTasks = currentPipelineFilter === 'all' 
            ? allTasks 
            : allTasks.filter(t => t.status === currentPipelineFilter);

          const summaryEl = document.getElementById('pipeline-filter-summary');
          if (summaryEl) summaryEl.innerText = `Hiển thị ${filteredTasks.length} / ${allTasks.length} tác vụ`;

          // Store allTasks globally for detail view access
          window.currentPipelineTasks = allTasks;

          const container = document.getElementById('multi-show-pipeline-container');
          if (container) {
            const renderTaskRow = (task) => {
              const isDone = task.status === 'done';
              const isActive = task.status === 'active';

              let statusBorder = "border-zinc-800/80 bg-zinc-950/80";
              let badgeColor = "bg-zinc-900 border-zinc-800 text-zinc-400";
              let percentBadge = `<span class="px-2.5 py-1 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-500 font-mono font-bold text-xs">0%</span>`;
              let iconBox = `<div class="w-8 h-8 rounded-xl bg-zinc-900 border border-zinc-800 text-zinc-500 flex items-center justify-center font-mono font-bold text-xs shrink-0">${task.index || '•'}</div>`;

              if (isDone) {
                statusBorder = "border-zinc-800/80 bg-zinc-900/40 hover:border-emerald-500/40";
                badgeColor = "bg-emerald-500/10 border-emerald-500/30 text-emerald-400";
                percentBadge = `<span class="px-2.5 py-1 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 font-mono font-bold text-xs">✓ 100%</span>`;
                iconBox = `<div class="w-8 h-8 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 flex items-center justify-center font-bold text-xs shrink-0">✓</div>`;
              } else if (isActive) {
                statusBorder = "border-amber-500/40 bg-amber-500/[0.04] shadow-md shadow-amber-500/5";
                badgeColor = "bg-amber-500/20 border-amber-500/40 text-amber-400";
                percentBadge = `<span class="px-2.5 py-1 rounded-xl bg-amber-500 text-black font-mono font-bold text-xs shadow-md shadow-amber-500/20">${task.percent}%</span>`;
                iconBox = `<div class="w-8 h-8 rounded-xl bg-amber-500/20 border border-amber-500/40 text-amber-400 flex items-center justify-center font-bold text-xs shrink-0 animate-pulse">⚡</div>`;
              }

              return `
                <div onclick="openPipelineDetail(${task.index})" class="px-3.5 py-3 sm:px-4 sm:py-3.5 rounded-2xl border ${statusBorder} hover:bg-zinc-900/80 cursor-pointer transition flex flex-col sm:flex-row sm:items-center justify-between gap-3 group shadow-sm">
                  
                  <!-- Left Side: Icon + Title + Tags + Path -->
                  <div class="flex items-center gap-3 min-w-0 flex-1">
                    ${iconBox}
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-2 flex-wrap">
                        <span class="px-2 py-0.5 rounded-md font-mono text-[10px] font-semibold border ${badgeColor}">
                          ${task.type}
                        </span>
                        <h4 class="font-bold text-xs sm:text-sm text-white truncate group-hover:text-amber-400 transition">${task.title}</h4>
                        <span class="px-1.5 py-0.2 rounded bg-zinc-900 border border-zinc-800 text-zinc-400 text-[10px] font-mono">${task.format}</span>
                      </div>
                      <div class="text-[11px] text-zinc-400 mt-0.5 flex items-center gap-2 truncate">
                        <span class="truncate">📍 ${task.destination}</span>
                        <span class="text-zinc-600 shrink-0">•</span>
                        <span class="text-zinc-400 font-mono shrink-0">${task.size}</span>
                      </div>
                    </div>
                  </div>

                  <!-- Right Side: Mini Compact Dual Stream Status + Percent + Arrow -->
                  <div class="flex items-center justify-between sm:justify-end gap-3 shrink-0 pt-2 sm:pt-0 border-t sm:border-t-0 border-zinc-800/60">
                    <div class="flex items-center gap-1.5 text-[10px] font-mono">
                      <span class="px-2 py-0.5 rounded-md bg-blue-500/10 text-blue-400 border border-blue-500/20 flex items-center gap-1 font-semibold" title="Tiến độ Download">
                        <span>📥</span> ${task.dl_percent}%
                      </span>
                      <span class="px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1 font-semibold" title="Tiến độ Upload Drive">
                        <span>📤</span> ${task.ul_percent}%
                      </span>
                    </div>

                    <div class="flex items-center gap-2">
                      ${percentBadge}
                      <span class="text-zinc-500 group-hover:text-amber-400 text-xs font-bold transition">➔</span>
                    </div>
                  </div>

                </div>
              `;
            };

            container.innerHTML = filteredTasks.map(renderTaskRow).join('');
          }
        } catch (e) {
        console.error("Pipelines fetch error:", e);
      }

      // 2. Fetch TorBox Cloud Cache
      try {
        const res = await fetch('/api/torbox');
        const data = await res.json();
        if (data.data) {
          currentTorrents = data.data;
          applyTorboxFilter();
          const tbCount = document.getElementById('torbox-count');
          if (tbCount) {
            const counts = data.counts || { total: currentTorrents.length, active: 0, queued: 0 };
            tbCount.innerText = `${counts.total} Torrents (${counts.active} Sẵn Sàng, ${counts.queued} Hàng Đợi)`;
          }
          const badgeQueued = document.getElementById('tbfilter-queued');
          if (badgeQueued && data.counts) {
            badgeQueued.innerText = `⏳ Hàng Đợi (${data.counts.queued})`;
          }
          const badgeReady = document.getElementById('tbfilter-ready');
          if (badgeReady && data.counts) {
            badgeReady.innerText = `🟢 Sẵn Sàng (${data.counts.active})`;
          }
        }
      } catch (e) {
        console.error(e);
      }
    }

    let selectedPipelineTask = null;
    let pipelineEpisodesData = [];


export {
  fetchData
};
