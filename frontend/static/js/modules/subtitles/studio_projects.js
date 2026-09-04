/**
 * Subtitle Studio Projects Tracker & Batch Controller
 */
import { showToast } from '../../core/toast.js';

    function switchSubTab(tab) {
      ['extractor', 'webvtt', 'translate'].forEach(t => {
        const btn = document.getElementById(`subtab-btn-${t}`);
        const content = document.getElementById(`subtab-content-${t}`);
        if (btn) {
          if (t === tab) {
            btn.className = "px-3.5 py-1.5 rounded-xl bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 font-bold transition";
          } else {
            btn.className = "px-3.5 py-1.5 rounded-xl bg-zinc-950 border border-zinc-800 text-zinc-400 hover:text-white transition";
          }
        }
        if (content) {
          content.classList.toggle('hidden', t !== tab);
        }
      });
      if (tab === 'translate') {
        loadSubtitleTranslationProjects();
      }
    }

    window.activeTranslatingBatches = window.activeTranslatingBatches || new Set();

    function sendTranslateBatchToAgent(showTitle, btnId) {
      if (window.quotaGuardLocked) {
        const q = window.quotaGuardStatus;
        const msg = q ? (q.day.used >= q.day.limit ? `Hôm nay đã dịch chạm trần ${q.day.used}/${q.day.limit} tập. Quota sẽ reset sau ${q.day.reset_in}.` : `Tuần này đã dịch chạm trần ${q.week.used}/${q.week.limit} tập (40% Quota Budget).`) : 'Translation Quota Guard đã tạm khóa để bảo vệ tài khoản.';
        showToast(`🛑 ${msg}`, 'warning', 5000);
        return;
      }

      const showKey = (showTitle || '').trim().toLowerCase();
      if (window.activeTranslatingBatches.has(showKey)) {
        return;
      }

      window.activeTranslatingBatches.add(showKey);
      
      const btn = btnId ? document.getElementById(btnId) : null;
      if (btn) {
        btn.disabled = true;
        btn.className = "px-4 py-2 bg-amber-500/20 text-amber-400 border border-amber-500/30 font-bold text-xs rounded-xl cursor-not-allowed flex items-center gap-1.5 shrink-0 animate-pulse";
        btn.innerHTML = `<span>⏳</span> Đang Dịch...`;
      }

      // Re-render UI immediately to highlight the translating episode badges
      loadSubtitleStudioData();
      loadSubtitleTranslationProjects();

      const mediaId = 'media-show-' + (showTitle || '').toLowerCase().replace(/[^a-z0-9]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
      sendQuickCommand(`translate-subtitle tự động chia batch tối đa 5 tập và dịch tuần tự tất cả các tập còn thiếu cho ${showTitle}. BẮT BUỘC in log console chi tiết từng bước [BƯỚC 1/5: Nạp nguồn], [BƯỚC 2/5: Glossary], [BƯỚC 3/5: Dịch thuật], [BƯỚC 4/5: Xuất bản], [BƯỚC 5/5: Audit] cho từng tập để người dùng theo dõi thời gian thực.`, mediaId);
    }

    async function syncShowSubtitles(showTitle, btn) {
      const icon = btn ? btn.querySelector('.sync-icon') : null;
      if (icon) icon.classList.add('animate-spin');
      if (btn) btn.disabled = true;
      try {
        const res = await fetch('/api/subtitles/sync', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({title: showTitle})
        });
        const data = await res.json();
        if (data.success) {
          if (!data.message.includes('đã có đầy đủ') && !data.message.includes('đã được đồng bộ')) {
            showToast(`⚡ ${data.message}`, 'success', 2000);
          }
        } else {
          showToast(`Lỗi đồng bộ: ${data.error || 'Thất bại'}`, 'error', 3500);
        }
      } catch (e) {
        showToast(`Lỗi kết nối: ${e.message}`, 'error', 3500);
      } finally {
        if (icon) icon.classList.remove('animate-spin');
        if (btn) btn.disabled = false;
      }
    }

    function toggleCardCollapse(contentId, btn) {
      const el = document.getElementById(contentId);
      if (!el) return;
      const isHidden = el.classList.contains('hidden');
      const icon = btn ? btn.querySelector('.toggle-icon') : null;
      if (isHidden) {
        el.classList.remove('hidden');
        if (icon) {
          icon.classList.remove('-rotate-90');
          icon.classList.add('rotate-0');
        }
      } else {
        el.classList.add('hidden');
        if (icon) {
          icon.classList.remove('rotate-0');
          icon.classList.add('-rotate-90');
        }
      }
    }

    async function loadSubtitleTranslationProjects() {
      const box = document.getElementById('sub-translation-projects-list');
      if (!box) return;
      if (typeof window.loadQuotaGuardStatus === 'function') window.loadQuotaGuardStatus();

      try {
        const res = await fetch('/api/subtitles/projects');
        if (!res.ok) throw new Error(`Máy chủ phản hồi mã lỗi HTTP ${res.status}`);
        const data = await res.json();
        const projects = data.projects || [];
        projects.sort((a, b) => {
          const aDone = (a.percent >= 100) ? 1 : 0;
          const bDone = (b.percent >= 100) ? 1 : 0;
          if (aDone !== bDone) return aDone - bDone;
          return (b.percent || 0) - (a.percent || 0);
        });

        if (projects.length === 0) {
          box.innerHTML = '<div class="p-4 text-center text-zinc-500 text-xs">Chưa có dự án dịch thuật phụ đề nào trong workspace.</div>';
          return;
        }

        box.innerHTML = projects.map((p, idx) => {
          const showKey = (p.title || p.name || '').trim().toLowerCase();
          const pendingEps = (p.episodes || []).filter(e => !e.vi_ass && !e.vi_srt && !e.vi_vtt).map(e => e.key);
          const isComplete = p.percent >= 100;
          const isRunning = window.activeTranslatingBatches.has(showKey);
          const singleEp = window.activeTranslatingEpisodes ? window.activeTranslatingEpisodes.get(showKey) : null;
          let activeBatchSet = new Set();
          if (singleEp) {
            activeBatchSet.add(singleEp);
          } else if (isRunning) {
            activeBatchSet = new Set(pendingEps.slice(0, 5));
          }
          const btnId = `btn-modal-trans-${idx}`;
          const collapseId = `modal-collapse-${idx}`;

          const episodesSectionHtml = renderEpisodesGridBySeason(p.episodes, activeBatchSet, p.title, `modal-show-${idx}`);

          return `
            <div class="p-3.5 rounded-2xl bg-zinc-950 border border-zinc-800 space-y-2.5 text-left" id="modal-show-card-${idx}">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-bold text-white text-xs truncate">${p.title}</span>
                    ${p.tvdb_id ? `<span class="px-1.5 py-0.5 rounded text-[9px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20">tvdb-${p.tvdb_id}</span>` : ''}
                    ${p.has_glossary ? `<span class="px-1.5 py-0.5 rounded text-[9px] font-mono bg-purple-500/10 text-purple-400 border border-purple-500/20">📑 Glossary</span>` : ''}
                  </div>
                  <div class="text-[10px] text-zinc-400 mt-0.5 font-mono">
                    Tiến độ: <strong class="${isComplete ? 'text-emerald-400' : 'text-amber-400'}">${p.completed_episodes} / ${p.total_episodes} tập</strong> (${p.percent}%)
                  </div>
                </div>
                <div class="flex items-center gap-1.5 shrink-0">
                  <button onclick="syncShowSubtitles('${p.title.replace(/'/g, "\\'")}', this)" class="px-2.5 py-1 bg-blue-600/20 text-blue-400 hover:bg-blue-600 hover:text-white border border-blue-500/30 rounded-xl text-xs font-semibold transition flex items-center gap-1 shadow-sm" title="Tự động đồng bộ các file phụ đề lên NAS Storage và Google Drive">
                    <span class="sync-icon">⚡</span> <span>Sync</span>
                  </button>
                  ${isComplete ? `
                    <span class="px-2.5 py-1 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 text-xs font-bold shrink-0">
                      🎉 Hoàn Tất
                    </span>
                  ` : (isRunning ? `
                    <button disabled class="px-3 py-1.5 bg-amber-500/20 text-amber-400 border border-amber-500/30 font-bold text-xs rounded-xl cursor-not-allowed flex items-center gap-1 shrink-0 animate-pulse">
                      <span>⏳</span> Đang Dịch...
                    </button>
                  ` : (pendingEps.length > 0 ? (window.quotaGuardLocked ? `
                    <button disabled class="px-3 py-1.5 bg-red-500/10 text-red-400 border border-red-500/20 font-bold text-xs rounded-xl cursor-not-allowed flex items-center gap-1 shrink-0" title="Translation Quota Guard tạm khóa để bảo vệ tài khoản">
                      <span>🛑</span> Quota Khóa
                    </button>
                  ` : `
                    <button id="${btnId}" onclick="sendTranslateBatchToAgent('${p.title.replace(/'/g, "\\'")}', '${btnId}')" class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs rounded-xl transition flex items-center gap-1 shrink-0 shadow-md shadow-emerald-600/20">
                      <span>🚀</span> Dịch Phụ Đề
                    </button>
                  `) : ''))}
                  <button onclick="toggleCardCollapse('${collapseId}', this)" class="p-1.5 rounded-xl bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-xs transition flex items-center justify-center" title="Thu gọn / Mở rộng danh sách tập">
                    <span class="toggle-icon inline-block text-[10px] transform transition-transform duration-200 ${isComplete ? '-rotate-90' : 'rotate-0'}">▼</span>
                  </button>
                </div>
              </div>

              <!-- Progress Bar -->
              <div class="w-full bg-zinc-900 rounded-full h-2 overflow-hidden border border-zinc-800">
                <div class="h-full rounded-full transition-all duration-500 ${isComplete ? 'bg-emerald-500' : 'bg-gradient-to-r from-emerald-500 to-amber-500'}" style="width: ${Math.max(p.percent, 2)}%"></div>
              </div>

              <!-- Episodes Grid by Season (collapsible) -->
              <div id="${collapseId}" class="pt-1 transition-all duration-300 ${isComplete ? 'hidden' : ''}">
                ${episodesSectionHtml}
              </div>
            </div>
          `;
        }).join('');

      } catch (e) {
        box.innerHTML = `<div class="p-4 text-center text-red-400 text-xs">Lỗi tải tiến độ: ${e}</div>`;
      }
    }

    async function loadSubtitleStudioData() {
      const container = document.getElementById('sub-studio-projects-container');
      const statTotal = document.getElementById('sub-stat-total-projects');
      const statDone = document.getElementById('sub-stat-completed-eps');
      const statPending = document.getElementById('sub-stat-pending-eps');
      const badgeCount = document.getElementById('sub-projects-badge-count');
      const sidebarBadge = document.getElementById('sidebar-sub-count');
      if (typeof window.loadQuotaGuardStatus === 'function') window.loadQuotaGuardStatus();

      try {
        const res = await fetch('/api/subtitles/projects');
        if (!res.ok) throw new Error(`Máy chủ phản hồi mã lỗi HTTP ${res.status}`);
        const data = await res.json();
        const projects = data.projects || [];
        projects.sort((a, b) => {
          const aDone = (a.percent >= 100) ? 1 : 0;
          const bDone = (b.percent >= 100) ? 1 : 0;
          if (aDone !== bDone) return aDone - bDone;
          return (b.percent || 0) - (a.percent || 0);
        });

        let totalCompleted = 0;
        let totalEpisodes = 0;

        projects.forEach(p => {
          totalCompleted += p.completed_episodes;
          totalEpisodes += p.total_episodes;
        });

        const totalPending = Math.max(0, totalEpisodes - totalCompleted);

        if (statTotal) statTotal.textContent = projects.length;
        if (statDone) statDone.textContent = `${totalCompleted} tập`;
        if (statPending) statPending.textContent = `${totalPending} tập`;
        if (badgeCount) badgeCount.textContent = `${projects.length} Dự Án`;
        if (sidebarBadge) sidebarBadge.textContent = `${totalCompleted}/${totalEpisodes}`;

        if (!container) return;

        if (projects.length === 0) {
          container.innerHTML = '<div class="p-6 text-center text-zinc-500 text-xs">Chưa có dự án dịch thuật phụ đề nào trong workspace.</div>';
          return;
        }

        container.innerHTML = projects.map((p, idx) => {
          const showKey = (p.title || p.name || '').trim().toLowerCase();
          const pendingEps = (p.episodes || []).filter(e => !e.vi_ass && !e.vi_srt && !e.vi_vtt).map(e => e.key);
          const isComplete = p.percent >= 100;
          const isRunning = window.activeTranslatingBatches.has(showKey);
          const singleEp = window.activeTranslatingEpisodes ? window.activeTranslatingEpisodes.get(showKey) : null;
          let activeBatchSet = new Set();
          if (singleEp) {
            activeBatchSet.add(singleEp);
          } else if (isRunning) {
            activeBatchSet = new Set(pendingEps.slice(0, 5));
          }
          const btnId = `btn-studio-trans-${idx}`;
          const collapseId = `studio-collapse-${idx}`;

          const episodesSectionHtml = renderEpisodesGridBySeason(p.episodes, activeBatchSet, p.title, `studio-show-${idx}`);

          return `
            <div class="p-4 sm:p-5 rounded-2xl bg-zinc-950/90 border border-zinc-800/90 space-y-3 shadow-md hover:border-zinc-700 transition" id="studio-show-card-${idx}">
              <div class="flex items-start justify-between gap-3 flex-wrap sm:flex-nowrap">
                <div class="min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span class="font-bold text-white text-sm truncate">${p.title}</span>
                    ${p.tvdb_id ? `<span class="px-2 py-0.5 rounded-full text-[9px] font-mono bg-blue-500/10 text-blue-400 border border-blue-500/20 font-bold">tvdb-${p.tvdb_id}</span>` : ''}
                    ${p.has_glossary ? `<span class="px-2 py-0.5 rounded-full text-[9px] font-mono bg-purple-500/10 text-purple-400 border border-purple-500/20 font-bold">📑 Glossary Chốt</span>` : ''}
                    ${p.has_progress ? `<span class="px-2 py-0.5 rounded-full text-[9px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-bold">📋 PROGRESS.md</span>` : ''}
                  </div>
                  <div class="text-xs text-zinc-400 mt-1 font-mono flex items-center gap-2">
                    <span>Tiến độ: <strong class="${isComplete ? 'text-emerald-400' : 'text-amber-400'} font-bold">${p.completed_episodes} / ${p.total_episodes} tập</strong></span>
                    <span>•</span>
                    <span class="font-bold ${isComplete ? 'text-emerald-400' : 'text-blue-400'}">${p.percent}%</span>
                  </div>
                </div>

                <div class="flex items-center gap-2 shrink-0">
                  <button onclick="syncShowSubtitles('${p.title.replace(/'/g, "\\'")}', this)" class="px-3 py-1.5 bg-blue-600/20 text-blue-400 hover:bg-blue-600 hover:text-white border border-blue-500/30 rounded-xl text-xs font-semibold transition flex items-center gap-1.5 shadow-sm" title="Tự động đồng bộ các file phụ đề còn thiếu lên NAS Storage và Google Drive">
                    <span class="sync-icon">⚡</span> <span>Sync</span>
                  </button>

                  ${isComplete ? `
                    <span class="px-3 py-1.5 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 text-xs font-bold flex items-center gap-1">
                      <span>🎉</span> Trọn Bộ Vietsub
                    </span>
                  ` : (isRunning ? `
                    <button disabled class="px-4 py-2 bg-amber-500/20 text-amber-400 border border-amber-500/30 font-bold text-xs rounded-xl cursor-not-allowed flex items-center gap-1.5 shrink-0 animate-pulse">
                      <span>⏳</span> Đang Dịch...
                    </button>
                  ` : (pendingEps.length > 0 ? (window.quotaGuardLocked ? `
                    <button disabled class="px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/20 font-bold text-xs rounded-xl cursor-not-allowed flex items-center gap-1.5 shrink-0" title="Translation Quota Guard tạm khóa để bảo vệ tài khoản">
                      <span>🛑</span> Quota Khóa
                    </button>
                  ` : `
                    <button id="${btnId}" onclick="sendTranslateBatchToAgent('${p.title.replace(/'/g, "\\'")}', '${btnId}')" class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs rounded-xl transition flex items-center gap-1.5 shadow-md shadow-emerald-600/20">
                      <span>🚀</span> Dịch Phụ Đề
                    </button>
                  `) : ''))}

                  <button onclick="toggleCardCollapse('${collapseId}', this)" class="p-2 rounded-xl bg-zinc-900 hover:bg-zinc-800 text-zinc-400 hover:text-white border border-zinc-800 text-xs font-semibold transition flex items-center justify-center" title="Thu gọn / Mở rộng danh sách tập">
                    <span class="toggle-icon inline-block text-[11px] transform transition-transform duration-200 ${isComplete ? '-rotate-90' : 'rotate-0'}">▼</span>
                  </button>
                </div>
              </div>

              <!-- Progress Bar -->
              <div class="w-full bg-zinc-900 rounded-full h-2.5 overflow-hidden border border-zinc-800 shadow-inner">
                <div class="h-full rounded-full transition-all duration-500 ${isComplete ? 'bg-emerald-500' : 'bg-gradient-to-r from-emerald-500 via-teal-500 to-amber-500'}" style="width: ${Math.max(p.percent, 2)}%"></div>
              </div>

              <!-- Episodes Grid by Season (collapsible) -->
              <div id="${collapseId}" class="pt-1 transition-all duration-300 ${isComplete ? 'hidden' : ''}">
                ${episodesSectionHtml}
              </div>
            </div>
          `;
        }).join('');

      } catch (e) {
        if (container) container.innerHTML = `<div class="p-6 text-center text-red-400 text-xs">Lỗi tải dữ liệu Subtitle Studio: ${e}</div>`;
      }
    }


export {
  switchSubTab,
  sendTranslateBatchToAgent,
  syncShowSubtitles,
  toggleCardCollapse,
  loadSubtitleTranslationProjects,
  loadSubtitleStudioData
};
