/**
 * Subtitle Extraction & WebVTT Staging Converter Tools
 */
import { showToast } from '../../core/toast.js';

    async function loadSubtitlesStaging() {
      const vBox = document.getElementById('sub-staging-videos-list');
      const sBox = document.getElementById('sub-staging-subs-list');
      const studioVBox = document.getElementById('sub-studio-staging-videos');
      const studioSBox = document.getElementById('sub-studio-staging-subs');

      if (vBox) vBox.innerHTML = '<div class="text-zinc-500 text-xs animate-pulse">⏳ Đang quét thư mục media_staging...</div>';
      if (sBox) sBox.innerHTML = '<div class="text-zinc-500 text-xs animate-pulse">⏳ Đang quét thư mục media_staging...</div>';
      if (studioVBox) studioVBox.innerHTML = '<div class="text-zinc-500 text-xs animate-pulse">⏳ Đang quét thư mục media_staging...</div>';
      if (studioSBox) studioSBox.innerHTML = '<div class="text-zinc-500 text-xs animate-pulse">⏳ Đang quét thư mục media_staging...</div>';

      try {
        const res = await fetch('/api/subtitles/staging');
        const data = await res.json();
        const files = data.files || [];

        const videos = files.filter(f => f.type === 'video');
        const subs = files.filter(f => f.type === 'subtitle');

        const renderVideosHtml = (vList) => {
          if (vList.length === 0) return '<div class="p-3 text-center text-zinc-500 text-xs">Không có file video nào trong staging.</div>';
          return vList.map(v => `
            <div class="p-2.5 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-between gap-2">
              <div class="min-w-0">
                <div class="font-bold text-white text-xs truncate">🎬 ${v.filename}</div>
                <div class="text-[10px] text-zinc-500 font-mono">${v.size_mb} MB • ${v.rel_path}</div>
              </div>
              <button onclick="extractSubtitlesFromVideo('${v.full_path.replace(/'/g, "\\'")}')" class="px-3 py-1 bg-emerald-600 hover:bg-emerald-500 text-white font-bold text-xs rounded-lg transition shrink-0">
                ✂️ Bóc Tách
              </button>
            </div>
          `).join('');
        };

        const renderSubsHtml = (sList) => {
          if (sList.length === 0) return '<div class="p-3 text-center text-zinc-500 text-xs">Không có file phụ đề rời nào trong staging.</div>';
          return sList.map(s => `
            <div class="p-2.5 rounded-xl bg-zinc-950 border border-zinc-800 flex items-center justify-between gap-2">
              <div class="min-w-0">
                <div class="font-bold text-white text-xs truncate">💬 ${s.filename}</div>
                <div class="text-[10px] text-zinc-500 font-mono">${s.size_mb} MB • ${s.rel_path}</div>
              </div>
              <button onclick="convertSubtitleToVtt('${s.full_path.replace(/'/g, "\\'")}')" class="px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white font-bold text-xs rounded-lg transition shrink-0">
                ⚡ Sang .vtt
              </button>
            </div>
          `).join('');
        };

        const vHtml = renderVideosHtml(videos);
        const sHtml = renderSubsHtml(subs);

        if (vBox) vBox.innerHTML = vHtml;
        if (sBox) sBox.innerHTML = sHtml;
        if (studioVBox) studioVBox.innerHTML = vHtml;
        if (studioSBox) studioSBox.innerHTML = sHtml;

      } catch (e) {
        if (vBox) vBox.innerHTML = `<div class="text-red-400 text-xs">Lỗi quét: ${e}</div>`;
        if (studioVBox) studioVBox.innerHTML = `<div class="text-red-400 text-xs">Lỗi quét: ${e}</div>`;
      }
    }

    async function extractSubtitlesFromVideo(filepath) {
      showToast('⏳ Đang bóc tách phụ đề nhúng qua ffmpeg...', 'info');
      try {
        const res = await fetch('/api/subtitles/extract', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({file: filepath})
        });
        const data = await res.json();
        if (data.success) {
          showToast('✅ ' + data.message, 'success');
          loadSubtitlesStaging();
        } else {
          showToast('❌ ' + (data.error || 'Lỗi bóc tách'), 'error');
        }
      } catch (e) {
        showToast('Lỗi: ' + e, 'error');
      }
    }

    async function convertSubtitleToVtt(filepath) {
      showToast('⏳ Đang chuẩn hóa typography sang WebVTT...', 'info');
      try {
        const res = await fetch('/api/subtitles/convert', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({file: filepath})
        });
        const data = await res.json();
        if (data.success) {
          showToast('✅ ' + data.message, 'success');
          loadSubtitlesStaging();
        } else {
          showToast('❌ ' + (data.error || 'Lỗi chuyển đổi'), 'error');
        }
      } catch (e) {
        showToast('Lỗi: ' + e, 'error');
      }
    }


export {
  loadSubtitlesStaging,
  extractSubtitlesFromVideo,
  convertSubtitleToVtt
};
