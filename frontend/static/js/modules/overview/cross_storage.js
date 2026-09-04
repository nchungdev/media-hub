/**
 * Cross Storage Scan & Proposal Executor (GDrive vs NAS vs Local)
 */
import { showToast } from '../../core/toast.js';

let crossStorageData = null;


    async function openCrossStorageModal() {
      openModal('modal-cross-storage');
      await scanAndCompareStorage();
    }

    async function scanAndCompareStorage() {
      const tbody = document.getElementById('cross-storage-tbody');
      if (tbody) {
        tbody.innerHTML = `
          <tr>
            <td colspan="5" class="p-8 text-center text-zinc-500">
              <div class="animate-pulse flex items-center justify-center gap-2">
                <span class="text-base">⏳</span> Đang kết nối SSH NAS & Google Drive để đối chiếu kho phim...
              </div>
            </td>
          </tr>
        `;
      }

      try {
        const res = await fetch('/api/library/cross_check', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'}
        });
        const data = await res.json();
        if (data.success) {
          crossStorageData = data;
          renderCrossStorageResults(data);
        } else {
          if (tbody) tbody.innerHTML = `<tr><td colspan="5" class="p-8 text-center text-red-400">❌ Lỗi: ${data.error || 'Không thể quét kho'}</td></tr>`;
        }
      } catch (e) {
        if (tbody) tbody.innerHTML = `<tr><td colspan="5" class="p-8 text-center text-red-400">❌ Lỗi kết nối: ${e}</td></tr>`;
      }
    }

    function renderCrossStorageResults(data) {
      const { summary, shows } = data;
      
      // Update KPIs
      if (document.getElementById('cross-total-shows')) document.getElementById('cross-total-shows').innerText = summary.total_shows;
      if (document.getElementById('cross-synced-both')) document.getElementById('cross-synced-both').innerText = summary.synced_both;
      if (document.getElementById('cross-only-gdrive')) document.getElementById('cross-only-gdrive').innerText = summary.only_gdrive;
      if (document.getElementById('cross-only-nas')) document.getElementById('cross-only-nas').innerText = summary.only_nas;

      const tbody = document.getElementById('cross-storage-tbody');
      if (!tbody) return;

      tbody.innerHTML = shows.map(s => {
        const gdriveBadge = s.in_gdrive ? 
          `<span class="px-2.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-bold text-[10px]">🟢 Có Sẵn</span>` :
          `<span class="px-2.5 py-0.5 rounded-full bg-zinc-800 text-zinc-500 border border-zinc-700 text-[10px]">⚪ Chưa Có</span>`;

        const nasBadge = s.in_nas ? 
          `<span class="px-2.5 py-0.5 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/20 font-bold text-[10px]">🟢 Có Sẵn</span>` :
          `<span class="px-2.5 py-0.5 rounded-full bg-zinc-800 text-zinc-500 border border-zinc-700 text-[10px]">⚪ Chưa Có</span>`;

        const localBadge = s.in_local ?
          `<span class="px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20 text-[9px] font-mono">📁 Buffer</span>` :
          `<span class="text-zinc-600 text-[10px]">--</span>`;

        const proposalBtns = s.proposals.map(p => {
          if (p.action === 'sync_to_nas') {
            return `
              <button onclick="executeSingleProposal('${s.folder.replace(/'/g, "\'")}', '${p.action}')" class="px-2.5 py-1 bg-amber-600 hover:bg-amber-500 text-white rounded-lg transition font-semibold text-[11px] shadow-sm flex items-center gap-1 shrink-0" title="${p.desc}">
                <span>☁️➔🖥️</span> Sync NAS
              </button>
            `;
          } else if (p.action === 'sync_to_drive') {
            return `
              <button onclick="executeSingleProposal('${s.folder.replace(/'/g, "\'")}', '${p.action}')" class="px-2.5 py-1 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg transition font-semibold text-[11px] shadow-sm flex items-center gap-1 shrink-0" title="${p.desc}">
                <span>🖥️➔☁️</span> Sync Drive
              </button>
            `;
          } else if (p.action === 'translate_vietsub') {
            return `
              <button onclick="executeSingleProposal('${s.folder.replace(/'/g, "\'")}', '${p.action}')" class="px-2.5 py-1 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition font-semibold text-[11px] shadow-sm flex items-center gap-1 shrink-0" title="${p.desc}">
                <span>🇻🇳</span> Dịch Sub
              </button>
            `;
          } else {
            return `
              <span class="px-2.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20 text-[10px] font-bold">
                ✓ Hoàn Hảo
              </span>
            `;
          }
        }).join('');

        return `
          <tr class="hover:bg-zinc-800/40 transition">
            <td class="px-4 py-3">
              <div class="flex items-center gap-3">
                <div class="w-8 h-11 rounded-lg bg-zinc-800 bg-cover bg-center shrink-0 border border-zinc-700/60 shadow-sm flex items-center justify-center text-xs" style="background-image: url('${s.poster || ''}')">
                  ${!s.poster ? '🎬' : ''}
                </div>
                <div class="min-w-0">
                  <div class="font-bold text-white text-xs truncate max-w-xs sm:max-w-md">${s.title}</div>
                  <div class="text-[10px] text-zinc-400 truncate">${s.vn || ''}</div>
                  <div class="text-[9px] text-zinc-500 font-mono mt-0.5">${s.folder}</div>
                </div>
              </div>
            </td>
            <td class="px-3 py-3 text-center">${gdriveBadge}</td>
            <td class="px-3 py-3 text-center">${nasBadge}</td>
            <td class="px-3 py-3 text-center">${localBadge}</td>
            <td class="px-4 py-3 text-right">
              <div class="flex items-center justify-end gap-1.5 flex-wrap">
                ${proposalBtns}
              </div>
            </td>
          </tr>
        `;
      }).join('');
    }

    async function executeSingleProposal(folder, action) {
      showToast(`🚀 Đang kích hoạt tiến trình ${action} cho "${folder}"...`, 'info', 2500);
      try {
        const res = await fetch('/api/agent/command', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({
            command: `Thực thi đề xuất ${action} cho bộ phim: ${folder}`
          })
        });
        const data = await res.json();
        if (data.success) {
          showToast(`✅ Đã gửi lệnh thực thi thành công!`, 'success');
        } else {
          showToast(`❌ ${data.error || 'Lỗi gửi lệnh'}`, 'error');
        }
      } catch (e) {
        showToast(`❌ Lỗi kết nối: ${e}`, 'error');
      }
    }

    async function executeAllProposals() {
      if (!crossStorageData || !crossStorageData.shows) return;
      const count = crossStorageData.summary.only_gdrive + crossStorageData.summary.only_nas;
      if (!confirm(`Bạn có chắc muốn tự động thực thi toàn bộ ${count} đề xuất đồng bộ giữa các kho lưu trữ?`)) return;
      showToast(`🚀 Đang gửi toàn bộ danh sách đề xuất cho AI Agent điều phối...`, 'info', 3000);
      try {
        const res = await fetch('/api/agent/command', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({
            command: `Tự động đồng bộ toàn diện tất cả các phim giữa Google Drive và NAS Storage để đạt 100% hoàn hảo`
          })
        });
        const data = await res.json();
        if (data.success) {
          showToast(`✅ Đã khởi động chuỗi đồng bộ toàn diện thành công!`, 'success');
          closeModal('modal-cross-storage');
        }
      } catch (e) {
        showToast(`❌ Lỗi kết nối: ${e}`, 'error');
      }
    }

export {
  crossStorageData,
  openCrossStorageModal,
  scanAndCompareStorage,
  renderCrossStorageResults,
  executeSingleProposal,
  executeAllProposals
};
