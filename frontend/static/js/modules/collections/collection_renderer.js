/**
 * Unified Media Collections Card Renderer, Franchise Grouping & Infinite Scroll
 */
import { showToast } from '../../core/toast.js';

function escapeHtml(str) {
  if (!str) return '';
  return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function escapeAttr(str) {
  if (!str) return '';
  return String(str).replace(/'/g, "\\'").replace(/"/g, '&quot;');
}

function hashString(str) {
  let hash = 0;
  const s = String(str || '');
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash) + s.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash).toString(36);
}

function renderSingleCollectionCard(col) {
  const isExpanded = window.expandedCollectionId === col.id;

  // 3 Pillar Interactive Badges - Chỉ hiện những cái tồn tại, nếu không có thì ẩn luôn
  let dlBadge = '';
  if (col.download?.state === 'complete') {
    dlBadge = `<span class="px-2 py-0.5 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ${col.download.label || 'Đã tải Draft'}</span>`;
  } else if (col.download?.state === 'partial') {
    dlBadge = `<span class="px-2 py-0.5 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20 text-xs font-mono flex items-center gap-1.5 animate-pulse"><span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span> ${col.download.label}</span>`;
  }

  let syncBadge = '';
  if (col.sync?.state === 'synced_both') {
    syncBadge = `<span class="px-2 py-0.5 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span> ${col.sync.label}</span>`;
  } else if (col.sync?.state === 'only_nas' || (col.sync?.in_nas && !col.sync?.in_gdrive)) {
    syncBadge = `<span class="px-2 py-0.5 rounded-lg bg-amber-500/10 text-amber-400 border border-amber-500/20 text-xs font-mono">🖥️ JellyPlex</span>`;
  } else if (col.sync?.state === 'only_gdrive' || (col.sync?.in_gdrive && !col.sync?.in_nas)) {
    syncBadge = `<span class="px-2 py-0.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20 text-xs font-mono">☁️ Drive</span>`;
  }

  let subBadge = '';
  if (col.subtitle?.state === 'complete') {
    subBadge = `<span class="px-2 py-0.5 rounded-lg bg-purple-500/10 text-purple-300 border border-purple-500/20 text-xs font-mono flex items-center gap-1.5"><span class="w-1.5 h-1.5 rounded-full bg-purple-400"></span> ${col.subtitle.label}</span>`;
  } else if (col.subtitle?.state === 'translating') {
    subBadge = `<button onclick="sendTranslateBatchToAgent('${escapeAttr(col.title)}', 'btn-col-trans-${col.id}')" id="btn-col-trans-${col.id}" class="px-2 py-0.5 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30 text-xs font-mono transition cursor-pointer flex items-center gap-1.5 animate-pulse" title="Bấm để tiếp tục dịch"><span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span> ${col.subtitle.label}</button>`;
  }

  // Expanded Detail Section with 6-Column Standardized Table
  let detailHtml = '';
  if (isExpanded) {
    let seasons = (col.seasons && col.seasons.length > 0) ? col.seasons : null;
    const isRemoteOnly = !seasons;

    // Synthesize episodes if no local seasons exist
    if (!seasons) {
      if (col.type === 'movie') {
        seasons = [{
          season_num: 1,
          name: 'Bản Chiếu (Movie)',
          episodes: [{
            key: 'movie-1',
            num: '1',
            name: col.title,
            video: Boolean(col.download?.state === 'complete' || col.sync?.in_local),
            in_nas: Boolean(col.sync?.in_nas),
            in_gdrive: Boolean(col.sync?.in_gdrive),
            video_files: [
              {
                name: `${col.title}.mkv`,
                quality: '1080p',
                size_mb: 0,
                in_nas: Boolean(col.sync?.in_nas),
                in_gdrive: Boolean(col.sync?.in_gdrive)
              }
            ],
            subtitle_files: col.subtitle?.state === 'complete' ? [
              {
                name: `${col.title}.vi.srt`,
                lang: 'vi',
                format: 'srt',
                is_internal: false
              }
            ] : [
              {
                name: `${col.title}.en.srt`,
                lang: 'en',
                format: 'srt',
                is_internal: false
              }
            ]
          }]
        }];
      } else {
        const epCount = Math.max(1, Math.min(col.total_episodes || 12, 24));
        const synthEpisodes = [];
        for (let i = 1; i <= epCount; i++) {
          const epStr = String(i).padStart(2, '0');
          synthEpisodes.push({
            key: `S01E${epStr}`,
            num: String(i),
            name: `Tập ${i}`,
            video: Boolean(col.download?.state === 'complete' || col.sync?.in_local),
            in_nas: Boolean(col.sync?.in_nas),
            in_gdrive: Boolean(col.sync?.in_gdrive),
            video_files: [
              {
                name: `Tập ${i} (1080p)`,
                quality: '1080p',
                size_mb: 0,
                in_nas: Boolean(col.sync?.in_nas),
                in_gdrive: Boolean(col.sync?.in_gdrive)
              }
            ],
            subtitle_files: col.subtitle?.state === 'complete' ? [
              {
                name: `Tập ${i}.vi.srt`,
                lang: 'vi',
                format: 'srt',
                is_internal: false
              }
            ] : [
              {
                name: `Tập ${i}.en.srt`,
                lang: 'en',
                format: 'srt',
                is_internal: false
              }
            ]
          });
        }
        seasons = [{
          season_num: 1,
          name: 'Season 01',
          episodes: synthEpisodes
        }];
      }
    }

    const activeSeasonNum = window.selectedCollectionSeason[col.id] !== undefined 
      ? window.selectedCollectionSeason[col.id] 
      : (seasons[0]?.season_num ?? 1);
    const activeSeason = seasons.find(s => s.season_num === activeSeasonNum) || seasons[0];
    const episodes = activeSeason?.episodes || [];

    const seasonTabsHtml = seasons.length > 1 ? seasons.map(s => {
      const isSActive = s.season_num === activeSeasonNum;
      return `
        <button onclick="selectCollectionSeason('${col.id}', ${s.season_num})" class="px-3 py-1.5 rounded-xl text-xs font-bold transition shrink-0 ${isSActive ? 'bg-amber-500 text-black shadow-md' : 'bg-zinc-900 text-zinc-400 hover:text-white border border-zinc-800'}">
          ${escapeHtml(s.name)} (${s.episodes.length})
        </button>
      `;
    }).join('') : '';

    const episodesTableHtml = episodes.length === 0 
      ? `<div class="p-6 text-center text-zinc-500 text-xs">Chưa có tập phim nào trong mùa này.</div>`
      : `
        <div class="overflow-x-auto">
          <table class="w-full text-left text-xs text-zinc-300 border-collapse table-fixed">
            <thead class="bg-zinc-950 text-zinc-400 uppercase text-[10px] tracking-wider border-b border-zinc-800">
              <tr>
                <th class="px-3 py-2.5 text-center w-12 font-mono">#</th>
                <th class="px-3 py-2.5 w-44 sm:w-56 truncate">Tên</th>
                <th class="px-3 py-2.5 text-center w-20 sm:w-24">Draft</th>
                <th class="px-3 py-2.5 text-center w-20 sm:w-24">JellyPlex</th>
                <th class="px-3 py-2.5 text-center w-20 sm:w-24">Drive</th>
                <th class="px-3 py-2.5 text-right w-24 sm:w-28">Action</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-zinc-850">
              ${episodes.map(ep => {
                const epToken = 'ep_' + Math.abs(hashString(col.id + '_' + ep.key));

                const hasDraft = Boolean(ep.video || (col.download?.state === 'complete') || col.sync?.in_local);
                const hasNas = Boolean(ep.in_nas || col.sync?.in_nas);
                const hasDrive = Boolean(ep.in_gdrive || col.sync?.in_gdrive);

                // Episode number
                let epNumDisplay = '';
                if (ep.num !== undefined && ep.num !== null && String(ep.num).trim() !== '') {
                  const m = String(ep.num).match(/\d+/);
                  epNumDisplay = m ? parseInt(m[0], 10) : ep.num;
                } else {
                  const raw = String(ep.key || ep.name || '');
                  const m = raw.match(/(?:s\d+)?e(\d+)/i) || raw.match(/(?:ep|episode)\s*(\d+)/i) || raw.match(/[-_]\s*(\d+)/) || raw.match(/\b(\d+)\b/);
                  epNumDisplay = m ? parseInt(m[1] || m[0], 10) : (ep.key || '1');
                }

                const epTitle = (col.type === 'movie' && !ep.name.startsWith('Tập')) ? ep.name : `Tập ${epNumDisplay}`;

                // Badges on main row - Chỉ hiện Có nếu tồn tại, nếu không có thì để dấu gạch mờ
                const draftPill = hasDraft
                  ? `<span class="inline-flex items-center justify-center px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">Có</span>`
                  : `<span class="text-zinc-700 font-mono text-xs">—</span>`;

                const nasPill = hasNas
                  ? `<span class="inline-flex items-center justify-center px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">Có</span>`
                  : `<span class="text-zinc-700 font-mono text-xs">—</span>`;

                const drivePill = hasDrive
                  ? `<span class="inline-flex items-center justify-center px-2 py-0.5 rounded text-[10px] font-bold bg-blue-500/15 text-blue-400 border border-blue-500/30">Có</span>`
                  : `<span class="text-zinc-700 font-mono text-xs">—</span>`;

                // Sub-rows
                const videoFiles = (ep.video_files && ep.video_files.length > 0)
                  ? ep.video_files
                  : [{
                      name: ep.name || `${col.title}.mkv`,
                      quality: (ep.name.match(/\b(2160p|4k|1080p|720p|480p|bdrip|remux|web-?dl)\b/i) || ['1080p'])[0].toUpperCase(),
                      size_mb: 0,
                      in_nas: hasNas,
                      in_gdrive: hasDrive,
                    }];

                const subFiles = ep.subtitle_files || [];

                // 1. Video Sub-Rows
                const videoRowsHtml = videoFiles.map(vf => {
                  const vfName = vf.name || ep.name;
                  const qual = vf.quality || (vfName.match(/\b(2160p|4k|1080p|720p|480p|bdrip|remux|web-?dl)\b/i) || ['1080p'])[0].toUpperCase();
                  const vDraft = hasDraft
                    ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` 
                    : `<span class="text-zinc-600 text-[11px]">—</span>`;
                  const vNas = (vf.in_nas ?? hasNas) 
                    ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` 
                    : `<span class="text-zinc-600 text-[11px]">—</span>`;
                  const vDrive = (vf.in_gdrive ?? hasDrive) 
                    ? `<span class="text-blue-400 font-bold text-[11px]">CÓ</span>` 
                    : `<span class="text-zinc-600 text-[11px]">—</span>`;

                  return `
                    <tr class="subrow-${epToken} hidden bg-zinc-950/90 hover:bg-zinc-900/60 transition border-b border-zinc-900/50">
                      <td class="px-3 py-2 text-center text-zinc-600 font-mono text-xs">↳</td>
                      <td class="px-3 py-2 w-44 sm:w-56 truncate">
                        <div class="flex items-center gap-1.5 truncate" title="${escapeHtml(vfName)}">
                          <span class="text-xs shrink-0">🎬</span>
                          <span class="font-mono text-zinc-300 text-[11px] font-semibold truncate">Video ${escapeHtml(qual)}</span>
                          ${vf.size_mb > 0 ? `<span class="text-[9px] text-zinc-500 font-mono shrink-0">(${vf.size_mb}M)</span>` : ''}
                        </div>
                      </td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${vDraft}</td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${vNas}</td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${vDrive}</td>
                      <td class="px-3 py-2 text-right w-24 sm:w-28">
                        <div class="flex items-center justify-end gap-1">
                          ${hasDraft ? `
                            <button onclick="playEpisode('${encodeURIComponent(col.folder || col.title)}', '${encodeURIComponent(activeSeason?.name || 'Season 01')}', '${encodeURIComponent(vfName)}')" class="px-2 py-0.5 rounded-lg bg-emerald-600/20 hover:bg-emerald-600 text-[10px] font-bold text-emerald-400 hover:text-white border border-emerald-500/30 transition inline-flex items-center gap-1 shadow-sm cursor-pointer" title="Phát video này">
                              <span>▶</span> Xem
                            </button>
                          ` : `
                            <button onclick="openSyncMediaModal('${escapeAttr(col.id)}', '${escapeAttr(ep.key)}')" class="px-2 py-0.5 rounded-lg bg-blue-600/20 hover:bg-blue-600 text-[10px] font-bold text-blue-400 hover:text-white border border-blue-500/30 transition inline-flex items-center gap-1 shadow-sm cursor-pointer" title="Đồng bộ / Tải video">
                              <span>🔄</span> Chi tiết
                            </button>
                          `}
                        </div>
                      </td>
                    </tr>
                  `;
                }).join('');

                // 2. Subtitle Sub-Rows
                let subRowsHtml = '';
                if (subFiles.length > 0) {
                  subRowsHtml = subFiles.map(sf => {
                    const isInternal = sf.is_internal;
                    const isVi = sf.lang === 'vi';
                    const formatBadge = (sf.format || 'SUB').toUpperCase();

                    const sDraft = isInternal 
                      ? `<span class="text-indigo-400 font-bold text-[11px]">Nhúng</span>`
                      : (hasDraft ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` : `<span class="text-zinc-600 text-[11px]">—</span>`);
                    const sNas = hasNas 
                      ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` 
                      : `<span class="text-zinc-600 text-[11px]">—</span>`;
                    const sDrive = hasDrive 
                      ? `<span class="text-blue-400 font-bold text-[11px]">CÓ</span>` 
                      : `<span class="text-zinc-600 text-[11px]">—</span>`;

                    return `
                      <tr class="subrow-${epToken} hidden bg-zinc-950/90 hover:bg-zinc-900/60 transition border-b border-zinc-900/50">
                        <td class="px-3 py-2 text-center text-zinc-600 font-mono text-xs">↳</td>
                        <td class="px-3 py-2 w-44 sm:w-56 truncate">
                          <div class="flex items-center gap-1.5 truncate" title="${escapeHtml(sf.name)}">
                            <span class="px-1 py-0.5 rounded text-[9px] font-mono font-bold ${isVi ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30' : 'bg-zinc-800 text-zinc-300 border border-zinc-700'} uppercase shrink-0">
                              .${escapeHtml(formatBadge)}
                            </span>
                            <span class="font-mono text-zinc-300 text-[11px] truncate">${escapeHtml(sf.name)}</span>
                            ${isInternal ? `<span class="px-1 py-0.2 rounded text-[8px] font-mono bg-indigo-500/20 text-indigo-300 shrink-0">Nhúng</span>` : ''}
                          </div>
                        </td>
                        <td class="px-3 py-2 text-center w-20 sm:w-24">${sDraft}</td>
                        <td class="px-3 py-2 text-center w-20 sm:w-24">${sNas}</td>
                        <td class="px-3 py-2 text-center w-20 sm:w-24">${sDrive}</td>
                        <td class="px-3 py-2 text-right w-24 sm:w-28">
                          <div class="flex items-center justify-end gap-1">
                            ${!isVi ? `
                              <button onclick="translateSingleSubtitle('${escapeAttr(col.title)}', '${escapeAttr(ep.key)}', '${escapeAttr(sf.name)}', ${Boolean(isInternal)})" class="px-2 py-0.5 rounded-lg bg-purple-600/20 hover:bg-purple-600 text-[10px] font-bold text-purple-300 hover:text-white border border-purple-500/30 transition inline-flex items-center gap-1 shadow-sm cursor-pointer" title="Dịch phụ đề này">
                                <span>🚀</span> Dịch
                              </button>
                            ` : `
                              ${!isInternal ? `
                                <a href="/api/subtitles/vtt?path=${encodeURIComponent(sf.path)}" target="_blank" class="px-2 py-0.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-[10px] font-semibold text-zinc-300 hover:text-white border border-zinc-700/80 transition inline-flex items-center gap-1 shadow-sm" title="Xem VTT">
                                  <span>👁️</span> VTT
                                </a>
                              ` : `<span class="text-purple-400 font-bold text-[10px] font-mono">✓ Vietsub</span>`}
                            `}
                          </div>
                        </td>
                      </tr>
                    `;
                  }).join('');
                } else {
                  // Fallback subtitle row
                  const isCompleteSub = col.subtitle?.state === 'complete';
                  const sDraft = (hasDraft && isCompleteSub) ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` : `<span class="text-zinc-600 text-[11px]">—</span>`;
                  const sNas = hasNas ? `<span class="text-emerald-400 font-bold text-[11px]">CÓ</span>` : `<span class="text-zinc-600 text-[11px]">—</span>`;
                  const sDrive = hasDrive ? `<span class="text-blue-400 font-bold text-[11px]">CÓ</span>` : `<span class="text-zinc-600 text-[11px]">—</span>`;

                  subRowsHtml = `
                    <tr class="subrow-${epToken} hidden bg-zinc-950/90 hover:bg-zinc-900/60 transition border-b border-zinc-900/50">
                      <td class="px-3 py-2 text-center text-zinc-600 font-mono text-xs">↳</td>
                      <td class="px-3 py-2 w-44 sm:w-56 truncate">
                        <div class="flex items-center gap-1.5 truncate">
                          <span class="px-1 py-0.5 rounded text-[9px] font-mono font-bold ${isCompleteSub ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30' : 'bg-zinc-800 text-zinc-400 border border-zinc-700'} uppercase shrink-0">.SRT</span>
                          <span class="font-mono text-zinc-400 text-[11px] truncate">${isCompleteSub ? 'vi subtitle.srt' : 'en subtitle.srt'}</span>
                        </div>
                      </td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${sDraft}</td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${sNas}</td>
                      <td class="px-3 py-2 text-center w-20 sm:w-24">${sDrive}</td>
                      <td class="px-3 py-2 text-right w-24 sm:w-28">
                        <div class="flex items-center justify-end gap-1">
                          ${isCompleteSub ? `
                            <span class="text-purple-400 font-bold text-[10px] font-mono">✓ Vietsub</span>
                          ` : `
                            <button onclick="sendTranslateBatchToAgent('${escapeAttr(col.title)}', 'btn-col-trans-${col.id}')" class="px-2 py-0.5 rounded-lg bg-purple-600/20 hover:bg-purple-600 text-[10px] font-bold text-purple-300 hover:text-white border border-purple-500/30 transition inline-flex items-center gap-1 shadow-sm cursor-pointer" title="Dịch phụ đề">
                              <span>🚀</span> Dịch
                            </button>
                          `}
                        </div>
                      </td>
                    </tr>
                  `;
                }

                const allSubRows = videoRowsHtml + subRowsHtml;

                return `
                  <!-- Main Episode Row -->
                  <tr class="hover:bg-zinc-900/60 transition cursor-pointer border-b border-zinc-800/80" onclick="toggleEpisodeFiles('${epToken}')">
                    <td class="px-3 py-2 font-mono font-bold text-amber-400 text-center w-12 text-xs">${epNumDisplay}</td>
                    <td class="px-3 py-2 w-44 sm:w-56 truncate">
                      <div class="font-bold text-white text-xs truncate" title="${escapeHtml(ep.name || epTitle)}">${epTitle}</div>
                    </td>
                    <td class="px-3 py-2 text-center w-20 sm:w-24">${draftPill}</td>
                    <td class="px-3 py-2 text-center w-20 sm:w-24">${nasPill}</td>
                    <td class="px-3 py-2 text-center w-20 sm:w-24">${drivePill}</td>
                    <td class="px-3 py-2 text-right w-24 sm:w-28" onclick="event.stopPropagation()">
                      <button onclick="toggleEpisodeFiles('${epToken}')" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-700/80 text-[11px] font-semibold transition inline-flex items-center gap-1 shadow-sm cursor-pointer ml-auto" title="Xem danh sách file của tập này">
                        <span>Chi tiết</span>
                        <span id="arrow-${epToken}" class="text-amber-400 font-bold transition">▾</span>
                      </button>
                    </td>
                  </tr>
                  <!-- Expanded Sub-Rows -->
                  ${allSubRows}
                `;
              }).join('')}
            </tbody>
          </table>
        </div>
      `;

    detailHtml = `
      <div class="mt-3 pt-3 border-t border-zinc-800/80 space-y-3 bg-zinc-950/60 p-3.5 sm:p-4 rounded-2xl">
        ${isRemoteOnly ? `
          <div class="flex items-center justify-between gap-3 p-3 rounded-xl bg-blue-950/30 border border-blue-500/20 text-xs flex-wrap">
            <div class="flex items-center gap-2 min-w-0">
              <span class="text-base shrink-0">☁️</span>
              <span class="text-zinc-300 truncate">Lưu trữ trên <strong class="${col.sync.color}">${col.sync.label}</strong>. Bấm Chi tiết trên từng tập để Xem / Tải về / Dịch.</span>
            </div>
            <button onclick="openSyncMediaModal('${escapeAttr(col.id)}', 'all')" class="px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition text-xs font-bold shadow-sm shrink-0 flex items-center gap-1.5 cursor-pointer ml-auto">
              <span>🔄</span> Đồng bộ tất cả
            </button>
          </div>
        ` : ''}

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

  // Show subtle franchise badge if part of a named franchise
  const showFranchiseBadge = col.franchise && col.franchise !== 'Chưa phân loại' && col.franchise !== col.title;

  return `
    <div class="p-3.5 sm:p-4 rounded-2xl bg-zinc-950/80 border border-zinc-800/80 hover:border-amber-500/30 transition shadow-lg space-y-2">
      <!-- Main Card Row -->
      <div class="flex items-center justify-between gap-4">
        <!-- Left: Poster + Title + Meta + 3 Interactive Pillars -->
        <div class="flex items-center gap-3.5 min-w-0 flex-1">
          <div class="w-14 h-20 sm:w-16 sm:h-24 rounded-xl bg-zinc-900 bg-cover bg-center shrink-0 border border-zinc-700/60 shadow-md flex items-center justify-center text-2xl overflow-hidden relative" style="background-image: url('${col.poster || ''}')">
            ${!col.poster ? '🎬' : ''}
          </div>
          <div class="space-y-1 min-w-0 flex-1">
            <!-- Title & Meta Line -->
            <div class="flex items-center gap-2 flex-wrap">
              <h3 class="font-bold text-sm sm:text-base text-white truncate cursor-pointer hover:text-amber-400 transition" onclick="toggleCollectionDetail('${col.id}')" title="${escapeHtml(col.title)}">
                ${escapeHtml(col.title)}
              </h3>
              <span class="text-[11px] text-zinc-500 font-mono shrink-0">
                ${col.type === 'movie' ? '🎬 Phim Lẻ' : '📺 Series'} ${col.year ? `• ${col.year}` : ''}
              </span>
              ${showFranchiseBadge ? `
                <span class="px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/20 text-[9px] font-mono shrink-0">
                  👑 ${escapeHtml(col.franchise)}
                </span>
              ` : ''}
            </div>
            
            <!-- 3 Pillar Interactive Badges (Chỉ hiện khi có ít nhất 1 badge tồn tại) -->
            ${(dlBadge || syncBadge || subBadge) ? `
              <div class="pt-0.5 flex items-center gap-1.5 flex-wrap">
                ${dlBadge}
                ${syncBadge}
                ${subBadge}
              </div>
            ` : ''}
          </div>
        </div>

        <!-- Right: Single Toggle Detail Button -->
        <div class="shrink-0">
          <button onclick="toggleCollectionDetail('${col.id}')" class="px-3 py-1.5 bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-800 text-xs font-semibold rounded-xl transition flex items-center gap-1.5 shadow-sm cursor-pointer">
            <span>${isExpanded ? '▲ Thu Gọn' : '▼ Chi Tiết'}</span>
          </button>
        </div>
      </div>

      <!-- Expanded Section -->
      ${detailHtml}
    </div>
  `;
}

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

  // 1. Group items by franchise
  const groupMap = new Map();
  const standaloneItems = [];

  for (const col of collections) {
    const rawF = (col.franchise && col.franchise.trim() !== '') ? col.franchise.trim() : 'Chưa phân loại';
    if (!groupMap.has(rawF)) {
      groupMap.set(rawF, {
        name: rawF,
        items: [],
        seriesCount: 0,
        movieCount: 0,
        hasDraft: false,
      });
    }
    const g = groupMap.get(rawF);
    g.items.push(col);
    if (col.type === 'movie') g.movieCount++;
    else g.seriesCount++;
    if (col.download && col.download.state === 'complete') g.hasDraft = true;
  }

  // Separate multi-item franchises and single titles
  const multiFranchises = [];
  for (const g of groupMap.values()) {
    if (g.items.length > 1 && g.name !== 'Chưa phân loại') {
      multiFranchises.push(g);
    } else {
      standaloneItems.push(...g.items);
    }
  }

  // Sort multi franchises: draft items first, then count desc, then name
  multiFranchises.sort((a, b) => {
    if (a.hasDraft !== b.hasDraft) return b.hasDraft ? 1 : -1;
    if (b.items.length !== a.items.length) return b.items.length - a.items.length;
    return a.name.localeCompare(b.name);
  });

  // Default all franchises to collapsed state
  window.expandedFranchises = window.expandedFranchises || new Set();

  // Form structured groups array: Multi franchises first, then standalone bundle
  const allGroups = [...multiFranchises];
  if (standaloneItems.length > 0) {
    allGroups.push({
      isStandaloneBundle: true,
      name: '__STANDALONE__',
      title: '🎬 Tác Phẩm Độc Lập / Chưa Gom Franchise',
      items: standaloneItems,
      seriesCount: standaloneItems.filter(i => i.type === 'series').length,
      movieCount: standaloneItems.filter(i => i.type === 'movie').length,
      hasDraft: standaloneItems.some(i => i.download && i.download.state === 'complete'),
    });
  }

  window.currentGroupKeys = allGroups.map(g => g.name);

  // 2. Pagination / Infinite Scroll slice
  const pageSize = window.colPageSize || 12;
  const maxGroups = (window.colPage || 1) * pageSize;
  const visibleGroups = allGroups.slice(0, maxGroups);
  const hasMore = visibleGroups.length < allGroups.length;
  window.hasMoreCollections = hasMore;

  // 3. Render Group Toolbar
  const toolbarHtml = `
    <div class="flex items-center justify-between gap-3 px-2 py-1 text-xs text-zinc-400 flex-wrap">
      <div class="flex items-center gap-2">
        <span>Hiển thị <strong class="text-amber-400 font-mono">${visibleGroups.length}</strong> / ${allGroups.length} nhóm (${collections.length} bộ phim)</span>
        ${multiFranchises.length > 0 ? `<span class="text-zinc-600">• ${multiFranchises.length} franchise lớn</span>` : ''}
      </div>
      <div class="flex items-center gap-1.5 ml-auto">
        <button onclick="collapseAllFranchises()" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-[11px] transition shadow-sm cursor-pointer">
          ▲ Thu gọn hết
        </button>
        <button onclick="expandAllFranchises()" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-[11px] transition shadow-sm cursor-pointer">
          ▼ Mở rộng hết
        </button>
      </div>
    </div>
  `;

  // 4. Render visible groups
  const groupsHtml = visibleGroups.map(g => {
    const isStandalone = Boolean(g.isStandaloneBundle);
    const isExpanded = window.expandedFranchises ? window.expandedFranchises.has(g.name) : false;
    const isCollapsed = !isExpanded;

    if (isStandalone) {
      const cardsHtml = !isCollapsed 
        ? g.items.map(col => renderSingleCollectionCard(col)).join('')
        : '';

      return `
        <div class="rounded-2xl bg-zinc-950/60 border border-zinc-800/80 p-3 sm:p-4 space-y-3 shadow-md">
          <!-- Standalone Header Bar -->
          <div class="flex items-center justify-between gap-3 pb-2 border-b border-zinc-800/60 cursor-pointer select-none" onclick="toggleFranchiseCollapse('${escapeAttr(g.name)}')">
            <div class="flex items-center gap-2.5 min-w-0 flex-1">
              <span class="w-8 h-8 rounded-lg bg-zinc-800 text-zinc-300 flex items-center justify-center font-bold text-sm border border-zinc-700 shrink-0">🎬</span>
              <div class="min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <h2 class="font-bold text-sm sm:text-base text-white hover:text-amber-400 transition truncate">${g.title}</h2>
                  <span class="px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-400 border border-zinc-700 text-[10px] font-mono font-bold shrink-0">
                    ${g.items.length} tác phẩm
                  </span>
                </div>
                <div class="text-[11px] text-zinc-500 font-mono flex items-center gap-2 flex-wrap pt-0.5">
                  <span>${g.seriesCount > 0 ? `${g.seriesCount} Series` : ''}${g.seriesCount > 0 && g.movieCount > 0 ? ' • ' : ''}${g.movieCount > 0 ? `${g.movieCount} Movies` : ''}</span>
                </div>
              </div>
            </div>

            <!-- Collapse / Expand Button -->
            <div class="shrink-0" onclick="event.stopPropagation()">
              <button onclick="toggleFranchiseCollapse('${escapeAttr(g.name)}')" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-xs font-semibold transition cursor-pointer flex items-center gap-1 shadow-sm">
                <span>${isCollapsed ? `▼ Mở rộng (${g.items.length})` : '▲ Thu gọn'}</span>
              </button>
            </div>
          </div>

          <!-- Standalone Items List -->
          ${!isCollapsed ? `
            <div class="space-y-3 pl-1 sm:pl-2.5 border-l-2 border-zinc-700/50 pt-1">
              ${cardsHtml}
            </div>
          ` : ''}
        </div>
      `;
    } else {
      // Major Franchise
      const cardsHtml = !isCollapsed 
        ? g.items.map(col => renderSingleCollectionCard(col)).join('')
        : '';

      const draftCount = g.items.filter(i => i.download && i.download.state === 'complete').length;
      const syncCount = g.items.filter(i => i.sync && i.sync.state === 'synced_both').length;

      return `
        <div class="franchise-group rounded-2xl bg-zinc-950/60 border border-zinc-800/80 p-3 sm:p-4 space-y-3 shadow-md">
          <!-- Franchise Header Bar -->
          <div class="flex items-center justify-between gap-3 pb-2 border-b border-zinc-800/60 cursor-pointer select-none" onclick="toggleFranchiseCollapse('${escapeAttr(g.name)}')">
            <div class="flex items-center gap-2.5 min-w-0 flex-1">
              <span class="w-8 h-8 rounded-lg bg-amber-500/10 text-amber-400 flex items-center justify-center font-bold text-sm border border-amber-500/20 shrink-0">👑</span>
              <div class="min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <h2 class="font-bold text-sm sm:text-base text-white hover:text-amber-400 transition truncate">${escapeHtml(g.name)}</h2>
                  <span class="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-400 border border-amber-500/30 text-[10px] font-mono font-bold shrink-0">
                    ${g.items.length} tác phẩm
                  </span>
                </div>
                <div class="text-[11px] text-zinc-500 font-mono flex items-center gap-2 flex-wrap pt-0.5">
                  <span>${g.seriesCount > 0 ? `${g.seriesCount} Series` : ''}${g.seriesCount > 0 && g.movieCount > 0 ? ' • ' : ''}${g.movieCount > 0 ? `${g.movieCount} Movies` : ''}</span>
                  ${draftCount > 0 ? `<span class="text-emerald-400 font-semibold">• Đã tải ${draftCount}/${g.items.length}</span>` : ''}
                  ${syncCount > 0 ? `<span class="text-blue-400 font-semibold">• Synced ${syncCount}/${g.items.length}</span>` : ''}
                </div>
              </div>
            </div>

            <!-- Collapse / Expand Button -->
            <div class="shrink-0" onclick="event.stopPropagation()">
              <button onclick="toggleFranchiseCollapse('${escapeAttr(g.name)}')" class="px-2.5 py-1 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-xs font-semibold transition cursor-pointer flex items-center gap-1 shadow-sm">
                <span>${isCollapsed ? `▼ Mở rộng (${g.items.length})` : '▲ Thu gọn'}</span>
              </button>
            </div>
          </div>

          <!-- Member Cards -->
          ${!isCollapsed ? `
            <div class="space-y-3 pl-1 sm:pl-2.5 border-l-2 border-amber-500/20 pt-1">
              ${cardsHtml}
            </div>
          ` : ''}
        </div>
      `;
    }
  }).join('');

  // 5. Infinite Scroll Sentinel / Load More Button
  const sentinelHtml = `
    <div id="collection-infinite-sentinel" class="py-6 flex flex-col items-center justify-center gap-2">
      ${hasMore ? `
        <button onclick="loadMoreCollections()" class="px-5 py-2.5 rounded-xl bg-zinc-900 hover:bg-zinc-800 text-zinc-300 hover:text-white border border-zinc-700/80 text-xs font-bold transition shadow-md flex items-center gap-2 cursor-pointer">
          <span>⏳ Tải thêm franchise (${visibleGroups.length} / ${allGroups.length} nhóm)</span>
        </button>
        <span class="text-[11px] text-zinc-500 font-mono">Cuộn xuống để tải thêm tự động</span>
      ` : `
        <div class="text-xs text-zinc-500 font-mono py-2">
          ✓ Đã hiển thị toàn bộ ${collections.length} tác phẩm (${allGroups.length} nhóm franchise)
        </div>
      `}
    </div>
  `;

  container.innerHTML = toolbarHtml + groupsHtml + sentinelHtml;

  // 6. Setup Infinite Scroll
  setupCollectionInfiniteScroll();
}

function setupCollectionInfiniteScroll() {
  if (window.colObserver) {
    window.colObserver.disconnect();
  }
  const sentinel = document.getElementById('collection-infinite-sentinel');
  if (!sentinel) return;

  const scrollContainer = document.querySelector('.overflow-y-auto.custom-scroll') || document.querySelector('main')?.parentElement;

  if ('IntersectionObserver' in window) {
    window.colObserver = new IntersectionObserver((entries) => {
      if (entries[0] && entries[0].isIntersecting) {
        if (window.hasMoreCollections) {
          loadMoreCollections();
        }
      }
    }, { 
      root: scrollContainer || null,
      rootMargin: '300px' 
    });
    window.colObserver.observe(sentinel);
  }

  // Direct scroll listener fail-safe on actual overflow scroll container
  if (scrollContainer && !scrollContainer._colScrollBound) {
    scrollContainer._colScrollBound = true;
    scrollContainer.addEventListener('scroll', () => {
      if (!window.hasMoreCollections) return;
      const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
      if (scrollHeight - (scrollTop + clientHeight) < 450) {
        loadMoreCollections();
      }
    }, { passive: true });
  }
}

function toggleEpisodeFiles(token) {
  const rows = document.querySelectorAll(`.subrow-${token}`);
  const arrow = document.getElementById(`arrow-${token}`);
  let isOpening = false;
  rows.forEach(r => {
    r.classList.toggle('hidden');
    isOpening = !r.classList.contains('hidden');
  });
  if (arrow) {
    arrow.innerText = isOpening ? '▴' : '▾';
  }
}

function toggleEpisodeSubtitles(colId, epKey) {
  const token = 'ep_' + Math.abs(hashString(colId + '_' + epKey));
  toggleEpisodeFiles(token);
}

function openDownloadSubtitlesModal(showTitle, epKey) {
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

// Global exposure
window.renderCollectionCards = renderCollectionCards;
window.renderSingleCollectionCard = renderSingleCollectionCard;
window.setupCollectionInfiniteScroll = setupCollectionInfiniteScroll;
window.openDownloadSubtitlesModal = openDownloadSubtitlesModal;
window.toggleEpisodeFiles = toggleEpisodeFiles;
window.toggleEpisodeSubtitles = toggleEpisodeSubtitles;

export {
  renderCollectionCards,
  renderSingleCollectionCard,
  setupCollectionInfiniteScroll,
  openDownloadSubtitlesModal,
  toggleEpisodeFiles,
  toggleEpisodeSubtitles
};


