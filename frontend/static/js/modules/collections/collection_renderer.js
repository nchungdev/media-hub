/**
 * Unified Media Collections Card Renderer & Subtitle Download
 */
import { showToast } from '../../core/toast.js';

    function renderCollectionCards(collections) {
      const container = document.getElementById('collections-list-container');
      if (!container) return;

      if (!collections || collections.length === 0) {
        container.innerHTML = `
          <div class="p-8 rounded-3xl bg-zinc-950/60 border border-zinc-800 text-center space-y-2">
            <span class="text-4xl block">🎬</span>
            <div class="text-white font-bold text-sm">Không tìm thấy bộ phim nào phù hợp</div>
            <p class="text-xs text-zinc-500">Thử thay đổi từ khóa tìm kiếm hoặc bộ lọc trạng thái phía trên.</p>
          </div>
        `;
        return;
      }

      container.innerHTML = collections.map(col => {
        const isExpanded = window.expandedCollectionId === col.id;
        const seasons = col.seasons || [];
        const activeSeasonNum = window.selectedCollectionSeason[col.id] !== undefined 
          ? window.selectedCollectionSeason[col.id] 
          : (seasons[0]?.season_num ?? 1);
        const activeSeason = seasons.find(s => s.season_num === activeSeasonNum) || seasons[0];

        // 3 Pillar Interactive Badges
        let dlBadge = `<span class="px-2.5 py-1 rounded-lg bg-zinc-900 text-zinc-400 border border-zinc-800 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-zinc-500"></span> ${col.download.label}</span>`;
        if (col.download.state === 'complete') {
          dlBadge = `<span class="px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ${col.download.label}</span>`;
        } else if (col.download.state === 'partial') {
          dlBadge = `<span class="px-2.5 py-1 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse"></span> ${col.download.label}</span>`;
        }

        let syncBadge = `<button onclick="syncShowSubtitles('${col.title}', this)" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-xs font-mono transition cursor-pointer flex items-center gap-1.5" title="Bấm để đồng bộ">${col.sync.label}</button>`;
        if (col.sync.state === 'synced_both') {
          syncBadge = `<span class="px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ${col.sync.label}</span>`;
        } else if (col.sync.state === 'only_nas') {
          syncBadge = `<span class="px-2.5 py-1 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20 text-xs font-mono">${col.sync.label}</span>`;
        } else if (col.sync.state === 'only_gdrive') {
          syncBadge = `<span class="px-2.5 py-1 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20 text-xs font-mono">${col.sync.label}</span>`;
        }

        let subBadge = `<button onclick="sendTranslateBatchToAgent('${col.title}', 'btn-col-trans-${col.id}')" id="btn-col-trans-${col.id}" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-purple-950 text-zinc-400 hover:text-purple-300 border border-zinc-800 hover:border-purple-500/30 text-xs font-mono transition cursor-pointer flex items-center gap-1.5" title="Bấm để bắt đầu dịch">${col.subtitle.label}</button>`;
        if (col.subtitle.state === 'complete') {
          subBadge = `<span class="px-2.5 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ${col.subtitle.label}</span>`;
        } else if (col.subtitle.state === 'translating') {
          subBadge = `<button onclick="sendTranslateBatchToAgent('${col.title}', 'btn-col-trans-${col.id}')" id="btn-col-trans-${col.id}" class="px-2.5 py-1 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30 text-xs font-mono transition cursor-pointer flex items-center gap-1.5 animate-pulse" title="Bấm để tiếp tục dịch">${col.subtitle.label}</button>`;
        }

        // Season tabs & episode table if expanded
        let detailHtml = '';
        if (isExpanded) {
          const seasonTabsHtml = seasons.map(s => {
            const isSActive = s.season_num === activeSeasonNum;
            return `
              <button onclick="selectCollectionSeason('${col.id}', ${s.season_num})" class="px-3 py-1.5 rounded-xl text-xs font-bold transition shrink-0 ${isSActive ? 'bg-amber-500 text-black shadow-md' : 'bg-zinc-900 text-zinc-400 hover:text-white border border-zinc-800'}">
                ${s.name} (${s.episodes.length})
              </button>
            `;
          }).join('');

          const episodes = activeSeason?.episodes || [];
          const episodesTableHtml = episodes.length === 0 
            ? `<div class="p-6 text-center text-zinc-500 text-xs">Chưa có tập phim nào trong mùa này.</div>`
            : `
              <div class="overflow-x-auto">
                <table class="w-full text-left text-xs text-zinc-300">
                  <thead class="bg-zinc-950 text-zinc-400 uppercase text-[10px] tracking-wider border-b border-zinc-800">
                    <tr>
                      <th class="px-3 py-2.5 w-16 sm:w-20">Tập</th>
                      <th class="px-3 py-2.5 max-w-[220px] sm:max-w-xs">Tên Tập</th>
                      <th class="px-3 py-2.5 text-center w-24">Local</th>
                      <th class="px-3 py-2.5 text-center w-24">NAS</th>
                      <th class="px-3 py-2.5 text-center w-28">Google Drive</th>
                      <th class="px-3 py-2.5 text-center w-36">Phụ Đề</th>
                      <th class="px-3 py-2.5 text-right w-28">Thao Tác</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-zinc-800/60">
                    ${episodes.map(ep => {
                      const subFiles = ep.subtitle_files || [];
                      const dlPill = ep.video 
                        ? `<span class="px-2 py-0.5 rounded-lg bg-emerald-500/10 text-emerald-400 font-bold text-[10px] border border-emerald-500/20">✓ Có Video</span>`
                        : `<span class="px-2 py-0.5 rounded-lg bg-zinc-900 text-zinc-500 text-[10px]">⚪ Đám Mây</span>`;
                      
                      const nasPill = ep.in_nas
                        ? `<span class="px-2 py-0.5 rounded-lg bg-emerald-500/10 text-emerald-400 font-bold text-[10px] border border-emerald-500/20">✓ Có Trên NAS</span>`
                        : `<span class="px-2 py-0.5 rounded-lg bg-zinc-900 text-zinc-500 text-[10px]">⚪ Chưa Có</span>`;

                      const drivePill = ep.in_gdrive
                        ? `<span class="px-2 py-0.5 rounded-lg bg-blue-500/10 text-blue-400 font-bold text-[10px] border border-blue-500/20">✓ Có Trên Drive</span>`
                        : `<span class="px-2 py-0.5 rounded-lg bg-zinc-900 text-zinc-500 text-[10px]">⚪ Chưa Có</span>`;

                      const langBadges = [...new Set(subFiles.map(s => s.lang.toUpperCase()))];
                      const langDisplay = langBadges.length > 0 
                        ? langBadges.map(l => `<span class="px-1.5 py-0.5 rounded text-[9px] font-bold ${l === 'VI' ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30' : 'bg-blue-500/20 text-blue-300 border border-blue-500/30'}">${l}</span>`).join(' ')
                        : '';

                      const subPill = subFiles.length > 0
                        ? `<button onclick="toggleEpisodeSubtitles('${col.id}', '${ep.key}')" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-700/80 font-mono text-[10px] transition flex items-center gap-1.5 mx-auto cursor-pointer shadow-sm" title="Bấm để xem danh sách ${subFiles.length} file phụ đề">
                            <span class="flex items-center gap-1">${langDisplay}</span>
                            <span class="text-zinc-500 font-bold">(${subFiles.length})</span>
                            <span id="arrow-sub-${col.id}-${ep.key}" class="text-amber-400 font-bold">▾</span>
                          </button>`
                        : `<span class="px-2 py-0.5 rounded-lg bg-zinc-900 text-zinc-500 text-[10px]">⚪ Chưa Có</span>`;

                      const subFilesHtml = subFiles.length > 0 ? `
                        <tr id="subfiles-${col.id}-${ep.key}" class="hidden bg-zinc-950/80 border-b border-zinc-800/40">
                          <td colspan="7" class="p-0">
                            <div class="bg-zinc-950/90 pl-3 sm:pl-6 border-l-2 border-purple-500/50">
                              <table class="w-full text-left text-xs text-zinc-400">
                                <tbody class="divide-y divide-zinc-900">
                                  ${subFiles.map(sf => {
                                    const isVi = sf.lang === 'vi';
                                    const langLabel = isVi ? 'VIETSUB' : sf.lang.toUpperCase();
                                    const formatBadge = sf.format.toUpperCase();
                                    return `
                                      <tr class="hover:bg-zinc-900/40 transition">
                                        <td class="px-3 py-1.5 w-16 sm:w-20 font-mono">
                                          <span class="px-1.5 py-0.5 rounded text-[9px] font-bold font-mono ${isVi ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30' : 'bg-blue-500/20 text-blue-300 border border-blue-500/30'} uppercase">
                                            .${formatBadge}
                                          </span>
                                        </td>
                                        <td class="px-3 py-1.5 max-w-[220px] sm:max-w-xs">
                                          <div class="font-mono text-zinc-300 text-[11px] truncate flex items-center gap-1.5" title="${sf.name}">
                                            <span>📄</span>
                                            <span class="truncate">${sf.name}</span>
                                          </div>
                                        </td>
                                        <td class="px-3 py-1.5 text-center w-24">
                                          <span class="px-1.5 py-0.5 rounded bg-zinc-900 text-zinc-400 font-mono text-[10px] border border-zinc-800">
                                            ${sf.size_kb} KB
                                          </span>
                                        </td>
                                        <td class="px-3 py-1.5 text-center w-24">
                                          ${ep.in_nas 
                                            ? `<span class="px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 font-mono text-[10px] border border-emerald-500/20">✓ Có</span>` 
                                            : `<span class="px-1.5 py-0.5 rounded bg-zinc-900 text-zinc-500 text-[10px]">⚪ Chưa</span>`}
                                        </td>
                                        <td class="px-3 py-1.5 text-center w-28">
                                          ${ep.in_gdrive 
                                            ? `<span class="px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-400 font-mono text-[10px] border border-blue-500/20">✓ Có</span>` 
                                            : `<span class="px-1.5 py-0.5 rounded bg-zinc-900 text-zinc-500 text-[10px]">⚪ Chưa</span>`}
                                        </td>
                                        <td class="px-3 py-1.5 text-center w-36">
                                          <span class="px-2 py-0.5 rounded-lg ${isVi ? 'bg-purple-500/10 text-purple-300 border border-purple-500/30' : 'bg-zinc-900 text-zinc-400 border border-zinc-800'} text-[10px] font-mono font-bold">
                                            ${langLabel}
                                          </span>
                                        </td>
                                        <td class="px-3 py-1.5 text-right w-28">
                                          <a href="/api/subtitles/vtt?path=${encodeURIComponent(sf.path)}" target="_blank" class="px-2 py-0.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-[10px] font-semibold text-zinc-300 hover:text-white border border-zinc-700/80 transition inline-flex items-center gap-1 shadow-sm" title="Xem định dạng WebVTT">
                                            <span>👁️</span> VTT
                                          </a>
                                        </td>
                                      </tr>
                                    `;
                                  }).join('')}
                                </tbody>
                              </table>
                            </div>
                          </td>
                        </tr>
                      ` : '';

                      return `
                        <tr class="hover:bg-zinc-900/60 transition">
                          <td class="px-3 py-2 font-mono font-bold text-amber-400 w-16 sm:w-20">${ep.key}</td>
                          <td class="px-3 py-2 max-w-[220px] sm:max-w-xs">
                            <div class="font-medium text-white truncate" title="${ep.name}">${ep.name}</div>
                          </td>
                          <td class="px-3 py-2 text-center w-24">${dlPill}</td>
                          <td class="px-3 py-2 text-center w-24">${nasPill}</td>
                          <td class="px-3 py-2 text-center w-28">${drivePill}</td>
                          <td class="px-3 py-2 text-center w-36">${subPill}</td>
                          <td class="px-3 py-2 text-right w-28">
                            <div class="flex items-center justify-end gap-1.5">
                              ${!ep.has_vi_sub ? `<button onclick="confirmTranslateSingleEpisode('${col.title}', '${ep.key}')" class="px-2.5 py-1 bg-purple-600/10 hover:bg-purple-600 text-purple-300 hover:text-white rounded-lg transition text-[10px] font-bold border border-purple-500/20">Dịch</button>` : ''}
                              ${ep.has_vi_sub ? `<button onclick="openDownloadSubtitlesModal('${col.title}', '${ep.key}')" class="px-2.5 py-1 bg-emerald-600/10 hover:bg-emerald-600 text-emerald-300 hover:text-white rounded-lg transition text-[10px] font-bold border border-emerald-500/20">Tải Sub</button>` : ''}
                            </div>
                          </td>
                        </tr>
                        ${subFilesHtml}
                      `;
                    }).join('')}
                  </tbody>
                </table>
              </div>
            `;

          detailHtml = `
            <div class="mt-3 pt-3 border-t border-zinc-800/80 space-y-3 bg-zinc-950/60 p-4 rounded-2xl">
              <div class="flex items-center justify-between gap-3 flex-wrap">
                <div class="flex items-center gap-1.5 overflow-x-auto custom-scroll">
                  ${seasonTabsHtml}
                </div>
                <div class="text-xs text-zinc-500">
                  Hiển thị <strong class="text-white">${episodes.length}</strong> tập
                </div>
              </div>
              <div class="bg-zinc-950 rounded-xl border border-zinc-800/80 overflow-hidden shadow-inner">
                ${episodesTableHtml}
              </div>
            </div>
          `;
        }

        return `
          <div class="p-3.5 sm:p-4 rounded-2xl bg-zinc-950/80 border border-zinc-800/80 hover:border-amber-500/30 transition shadow-lg space-y-2">
            <!-- Main Card Row -->
            <div class="flex items-center justify-between gap-4">
              <!-- Left: Poster + Title + Meta + 3 Interactive Pillars -->
              <div class="flex items-center gap-3.5 min-w-0 flex-1">
                <div class="w-14 h-20 sm:w-16 sm:h-24 rounded-xl bg-zinc-900 bg-cover bg-center shrink-0 border border-zinc-700/60 shadow-md flex items-center justify-center text-2xl overflow-hidden" style="background-image: url('${col.poster}')">
                  ${!col.poster ? '🎬' : ''}
                </div>
                <div class="space-y-1 min-w-0 flex-1">
                  <!-- Title & Meta Line -->
                  <div class="flex items-center gap-2 flex-wrap">
                    <h3 class="font-bold text-sm sm:text-base text-white truncate cursor-pointer hover:text-amber-400 transition" onclick="toggleCollectionDetail('${col.id}')" title="${col.title}">
                      ${col.title}
                    </h3>
                    <span class="text-[11px] text-zinc-500 font-mono shrink-0">
                      ${col.type === 'movie' ? '🎬 Phim Lẻ' : '📺 Series'} ${col.year ? `• ${col.year}` : ''}
                    </span>
                  </div>
                  
                  <!-- 3 Pillar Interactive Badges -->
                  <div class="pt-0.5 flex items-center gap-1.5 flex-wrap">
                    ${dlBadge}
                    ${syncBadge}
                    ${subBadge}
                  </div>
                </div>
              </div>

              <!-- Right: Single Toggle Detail Button -->
              <div class="shrink-0">
                <button onclick="toggleCollectionDetail('${col.id}')" class="px-3 py-1.5 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-800 text-xs font-semibold rounded-xl transition flex items-center gap-1.5 shadow-sm">
                  <span>${isExpanded ? '▲ Thu Gọn' : '▼ Chi Tiết'}</span>
                </button>
              </div>
            </div>

            <!-- Expanded Section -->
            ${detailHtml}
          </div>
        `;
      }).join('');
    }


export function openDownloadSubtitlesModal(showTitle, epKey) {
  const showEl = document.getElementById("download-sub-show-title");
  const epEl = document.getElementById("download-sub-ep-key");
  const listEl = document.getElementById("download-sub-files-list");
  if (showEl) showEl.textContent = showTitle || "--";
  if (epEl) epEl.textContent = epKey || "--";
  if (listEl) {
    const formats = ["ass", "srt", "vtt"];
    listEl.innerHTML = formats.map(fmt => `
      <a href="/api/subtitles/download?show=${encodeURIComponent(showTitle)}&ep=${encodeURIComponent(epKey)}&format=${fmt}"
         download="${showTitle}_${epKey}.vi.${fmt}"
         class="flex items-center justify-between px-3 py-2 rounded-xl bg-zinc-900 hover:bg-zinc-800 border border-zinc-700/60 text-xs text-white transition group">
        <span class="font-mono text-emerald-400 font-bold">.${fmt.toUpperCase()}</span>
        <span class="text-zinc-400 group-hover:text-emerald-300 transition">Tải về ⬇</span>
      </a>
    `).join("");
  }
  if (typeof window.openModal === "function") {
    window.openModal("modal-download-episode-sub");
  }
}

export {
  renderCollectionCards
};
