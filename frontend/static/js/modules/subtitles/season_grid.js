/**
 * Subtitle Studio Season Grid & Episode Matrix
 */
import { showToast } from '../../core/toast.js';

    function renderEpisodesGridBySeason(episodesList, activeBatchSet, showTitle, prefixId) {
      const seasonsMap = {};
      (episodesList || []).forEach(e => {
        let sNum = (typeof e.season_num === 'number') ? e.season_num : null;
        if (sNum === null) {
          const m = (e.key || '').match(/S(\d+)E(\d+)/i);
          sNum = m ? parseInt(m[1], 10) : 1;
        }
        const sName = (sNum === 0) ? 'Specials (Season 00)' : `Season ${String(sNum).padStart(2, '0')}`;
        if (!seasonsMap[sNum]) {
          seasonsMap[sNum] = { sNum, sName, episodes: [] };
        }
        seasonsMap[sNum].episodes.push(e);
      });

      const seasonKeys = Object.keys(seasonsMap).map(Number).sort((a, b) => a - b);
      if (seasonKeys.length === 0) {
        return '<div class="p-3 text-center text-zinc-500 text-xs">Chưa có dữ liệu tập phim.</div>';
      }

      const renderPills = (eps) => {
        return eps.map(e => {
          const hasSub = e.vi_ass || e.vi_srt || e.vi_vtt;
          const isTranslating = activeBatchSet.has(e.key);
          const epData = encodeURIComponent(JSON.stringify({
            showTitle: showTitle,
            epKey: e.key,
            hasSub: Boolean(hasSub),
            vi_ass: Boolean(e.vi_ass),
            vi_ass_path: e.vi_ass_path || '',
            vi_ass_name: e.vi_ass_name || '',
            vi_srt: Boolean(e.vi_srt),
            vi_srt_path: e.vi_srt_path || '',
            vi_srt_name: e.vi_srt_name || '',
            vi_vtt: Boolean(e.vi_vtt),
            vi_vtt_path: e.vi_vtt_path || '',
            vi_vtt_name: e.vi_vtt_name || ''
          }));

          if (isTranslating) {
            return `<span class="px-2.5 py-1 rounded-xl text-[11px] font-mono border bg-amber-500/20 text-amber-300 border-amber-500/50 animate-pulse font-bold flex items-center gap-1 shadow-md shadow-amber-500/20 cursor-wait" title="${e.key}: Đang dịch thuật AI...">🔄 ${e.key} (Đang dịch)</span>`;
          }
          if (hasSub) {
            return `<button onclick="openEpisodeActionModal('${epData}')" class="px-2 py-0.5 rounded-lg text-[10px] font-mono border bg-emerald-500/10 hover:bg-emerald-500/25 text-emerald-400 border-emerald-500/30 hover:border-emerald-400/50 flex items-center gap-1 transition cursor-pointer shadow-sm hover:scale-105" title="${e.key}: Đã có Vietsub - Bấm để tải về">✅ ${e.key}</button>`;
          }
          const bg = e.eng_sub ? 'bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border-zinc-800 hover:border-amber-500/40' : 'bg-zinc-950 hover:bg-zinc-900 text-zinc-600 hover:text-zinc-400 border-zinc-900';
          const icon = e.eng_sub ? '⏳' : '⚪';
          return `<button onclick="openEpisodeActionModal('${epData}')" class="px-2 py-0.5 rounded-lg text-[10px] font-mono border ${bg} flex items-center gap-1 transition cursor-pointer hover:scale-105 hover:border-amber-500/40" title="${e.key}: Chưa có Vietsub - Bấm để dịch tập này">${icon} ${e.key}</button>`;
        }).join('');
      };

      // If only 1 season, render single grid cleanly
      if (seasonKeys.length === 1) {
        const sObj = seasonsMap[seasonKeys[0]];
        return `
          <div class="flex items-center gap-1.5 flex-wrap max-h-28 overflow-y-auto custom-scroll p-1.5 bg-zinc-900/40 rounded-xl border border-zinc-900">
            ${renderPills(sObj.episodes)}
          </div>
        `;
      }

      // Multiple seasons: Render Season Tabs Header + Season Grids
      const seasonTabsHeader = `
        <div class="flex items-center gap-1.5 overflow-x-auto custom-scroll pb-1 mb-1.5">
          ${seasonKeys.map((sNum, sIdx) => {
            const sObj = seasonsMap[sNum];
            const sDone = sObj.episodes.filter(e => e.vi_ass || e.vi_srt || e.vi_vtt).length;
            const sTotal = sObj.episodes.length;
            const isAct = sIdx === 0;
            const btnClass = isAct ? 'bg-blue-600/20 text-blue-400 border-blue-500/40 font-bold shadow-sm' : 'bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-white hover:border-zinc-700';
            return `
              <button onclick="switchShowSeason('${prefixId}', ${sNum})" id="${prefixId}-stab-btn-${sNum}" class="px-2.5 py-1 rounded-xl text-[11px] border font-mono transition shrink-0 flex items-center gap-1.5 cursor-pointer ${btnClass}">
                <span>${sObj.sName}</span>
                <span class="text-[10px] opacity-75">(${sDone}/${sTotal})</span>
              </button>
            `;
          }).join('')}
        </div>
      `;

      const seasonGrids = seasonKeys.map((sNum, sIdx) => {
        const sObj = seasonsMap[sNum];
        const isAct = sIdx === 0;
        return `
          <div id="${prefixId}-season-grid-${sNum}" class="${isAct ? 'flex' : 'hidden'} items-center gap-1.5 flex-wrap max-h-28 overflow-y-auto custom-scroll p-1.5 bg-zinc-900/40 rounded-xl border border-zinc-900">
            ${renderPills(sObj.episodes)}
          </div>
        `;
      }).join('');

      return seasonTabsHeader + seasonGrids;
    }

    function switchShowSeason(prefixId, targetSNum) {
      const grids = document.querySelectorAll(`[id^="${prefixId}-season-grid-"]`);
      const btns = document.querySelectorAll(`[id^="${prefixId}-stab-btn-"]`);

      grids.forEach(el => {
        el.classList.add('hidden');
        el.classList.remove('flex');
      });
      btns.forEach(el => {
        el.className = "px-2.5 py-1 rounded-xl text-[11px] border font-mono transition shrink-0 flex items-center gap-1.5 bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-white hover:border-zinc-700 cursor-pointer";
      });

      const activeGrid = document.getElementById(`${prefixId}-season-grid-${targetSNum}`);
      const activeBtn = document.getElementById(`${prefixId}-stab-btn-${targetSNum}`);

      if (activeGrid) {
        activeGrid.classList.remove('hidden');
        activeGrid.classList.add('flex');
      }
      if (activeBtn) {
        activeBtn.className = "px-2.5 py-1 rounded-xl text-[11px] border font-mono transition shrink-0 flex items-center gap-1.5 bg-blue-600/20 text-blue-400 border-blue-500/40 font-bold shadow-sm cursor-pointer";
      }
    }

    window.currentSelectedEpisode = null;


export {
  renderEpisodesGridBySeason,
  switchShowSeason
};
