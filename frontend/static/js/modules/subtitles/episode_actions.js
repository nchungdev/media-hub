/**
 * Subtitle Single Episode Actions & Modals
 */
import { showToast } from '../../core/toast.js';

    function openEpisodeActionModal(encodedData) {
      try {
        const data = JSON.parse(decodeURIComponent(encodedData));
        window.currentSelectedEpisode = data;

        if (data.hasSub) {
          // Open Download Modal
          const showEl = document.getElementById('download-sub-show-title');
          const epEl = document.getElementById('download-sub-ep-key');
          if (showEl) showEl.textContent = data.showTitle || '--';
          if (epEl) epEl.textContent = data.epKey || '--';
          
          const list = document.getElementById('download-sub-files-list');
          let html = '';
          
          if (data.vi_ass_path) {
            html += `
              <div class="p-3 rounded-2xl bg-zinc-900/80 border border-zinc-800 flex items-center justify-between gap-3 hover:border-zinc-700 transition">
                <div class="min-w-0">
                  <div class="text-xs font-bold text-emerald-400 flex items-center gap-1.5">
                    <span>📄</span> Advanced SubStation (.vi.ass)
                  </div>
                  <div class="text-[10px] text-zinc-400 font-mono truncate mt-0.5">${data.vi_ass_name || 'Phụ đề typography nâng cao'}</div>
                </div>
                <a href="/api/subtitles/download?path=${encodeURIComponent(data.vi_ass_path)}" download class="px-3.5 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-xl text-xs font-bold transition flex items-center gap-1 shrink-0 shadow-md shadow-emerald-600/20">
                  <span>📥</span> Tải Về
                </a>
              </div>
            `;
          }
          if (data.vi_srt_path) {
            html += `
              <div class="p-3 rounded-2xl bg-zinc-900/80 border border-zinc-800 flex items-center justify-between gap-3 hover:border-zinc-700 transition">
                <div class="min-w-0">
                  <div class="text-xs font-bold text-blue-400 flex items-center gap-1.5">
                    <span>📝</span> SubRip Subtitle (.vi.srt)
                  </div>
                  <div class="text-[10px] text-zinc-400 font-mono truncate mt-0.5">${data.vi_srt_name || 'Phụ đề tương thích đa thiết bị'}</div>
                </div>
                <a href="/api/subtitles/download?path=${encodeURIComponent(data.vi_srt_path)}" download class="px-3.5 py-1.5 bg-blue-600 hover:bg-blue-500 text-white rounded-xl text-xs font-bold transition flex items-center gap-1 shrink-0 shadow-md shadow-blue-600/20">
                  <span>📥</span> Tải Về
                </a>
              </div>
            `;
          }
          if (data.vi_vtt_path) {
            html += `
              <div class="p-3 rounded-2xl bg-zinc-900/80 border border-zinc-800 flex items-center justify-between gap-3 hover:border-zinc-700 transition">
                <div class="min-w-0">
                  <div class="text-xs font-bold text-purple-400 flex items-center gap-1.5">
                    <span>🌐</span> WebVTT Stream (.vi.vtt)
                  </div>
                  <div class="text-[10px] text-zinc-400 font-mono truncate mt-0.5">${data.vi_vtt_name || 'Phụ đề Web streaming zero-latency'}</div>
                </div>
                <a href="/api/subtitles/download?path=${encodeURIComponent(data.vi_vtt_path)}" download class="px-3.5 py-1.5 bg-purple-600 hover:bg-purple-500 text-white rounded-xl text-xs font-bold transition flex items-center gap-1 shrink-0 shadow-md shadow-purple-600/20">
                  <span>📥</span> Tải Về
                </a>
              </div>
            `;
          }

          if (!html) {
            html = '<div class="p-4 text-center text-zinc-500 text-xs">File phụ đề đang nằm trong container video hoặc chưa sinh file rời.</div>';
          }

          if (list) list.innerHTML = html;
          openModal('modal-download-episode-sub');
        } else {
          // Open Confirm Translation Modal
          const showEl = document.getElementById('confirm-trans-show-title');
          const epEl = document.getElementById('confirm-trans-ep-key');
          if (showEl) showEl.textContent = data.showTitle || '--';
          if (epEl) epEl.textContent = data.epKey || '--';
          openModal('modal-confirm-translate-episode');
        }
      } catch (e) {
        console.error('Error opening episode modal:', e);
      }
    }

    function confirmTranslateSingleEpisode() {
      const data = window.currentSelectedEpisode;
      if (!data) return;

      if (window.quotaGuardLocked) {
        const q = window.quotaGuardStatus;
        const msg = q ? (q.day.used >= q.day.limit ? `Hôm nay đã dịch chạm trần ${q.day.used}/${q.day.limit} tập. Quota sẽ reset sau ${q.day.reset_in}.` : `Tuần này đã dịch chạm trần ${q.week.used}/${q.week.limit} tập.`) : 'Translation Quota Guard đã tạm khóa.';
        showToast(`🛑 ${msg}`, 'warning', 4000);
        closeModal('modal-confirm-translate-episode');
        return;
      }

      closeModal('modal-confirm-translate-episode');

      const showKey = (data.showTitle || '').trim().toLowerCase();
      window.activeTranslatingBatches = window.activeTranslatingBatches || new Set();
      window.activeTranslatingBatches.add(showKey);

      window.activeTranslatingEpisodes = window.activeTranslatingEpisodes || new Map();
      window.activeTranslatingEpisodes.set(showKey, data.epKey);

      // Re-render UI immediately to highlight ONLY the selected episode badge as (Đang dịch)
      loadSubtitleStudioData();
      loadSubtitleTranslationProjects();

      const mediaId = 'media-show-' + (data.showTitle || '').toLowerCase().replace(/[^a-z0-9]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '');
      const command = `translate-subtitle: DỊCH DUY NHẤT 1 TẬP. Yêu cầu tự động dịch phụ đề tiếng Việt chuyên sâu cho "${data.showTitle}" duy nhất tập ${data.epKey}. Chỉ dịch và xuất đúng 3 file (.vi.ass, .vi.srt, .vi.vtt) cho tập ${data.epKey}, tuyệt đối KHÔNG dịch bất kỳ tập nào khác.
QUY ĐỊNH LOGGING: BẮT BUỘC in ra console từng bước [BƯỚC 1/5: Khảo sát nguồn], [BƯỚC 2/5: Đối chiếu Glossary], [BƯỚC 3/5: Dịch thuật], [BƯỚC 4/5: Xuất bản 3 định dạng], [BƯỚC 5/5: Audit & Hoàn tất] để hiển thị thời gian thực lên Live Console.`;

      sendQuickCommand(command, mediaId);
      showToast(`🚀 Đã gửi lệnh dịch duy nhất tập ${data.epKey} sang AI Agent!`, 'success', 2500);
    }

    function sendSubStudioCommand() {
      const inputEl = document.getElementById('sub-studio-custom-prompt');
      const text = inputEl?.value?.trim();
      if (!text) {
        showToast('Vui lòng nhập nội dung câu lệnh dịch phụ đề', 'warning');
        return;
      }
      sendQuickCommand(text);
      if (inputEl) inputEl.value = '';
      showToast('🚀 Đã gửi lệnh dịch sang AI Agent xử lý nền!', 'success', 2000);
    }


    function sendSubTranslateToAgent() {
      const text = document.getElementById('sub-translate-input')?.value?.trim();
      if (!text) {
        showToast('Vui lòng nhập nội dung hoặc tên phim cần dịch', 'warning');
        return;
      }
      closeModal('modal-toolbox-subtitles');
      setTab('agent');
      sendQuickCommand(`translate-subtitle dịch thuật phụ đề tiếng Việt chuyên sâu cho: ${text}`);
    }



export {
  openEpisodeActionModal,
  confirmTranslateSingleEpisode,
  sendSubStudioCommand,
  sendSubTranslateToAgent
};
