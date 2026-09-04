/**
 * Artplayer Video Streaming & External App Player Launchers
 */
import { showToast } from '../../core/toast.js';

    let art = null;
    let activePlayingUrl = "";
    let currentPlayingContext = null;
    let currentSelectedBufferMb = localStorage.getItem('preferred_buffer_mb') || '32';

    function changePlayerBuffer(val) {
      currentSelectedBufferMb = val;
      try { localStorage.setItem('preferred_buffer_mb', val); } catch (e) {}
      if (currentPlayingContext) {
        const currentTime = art ? (art.currentTime || 0) : 0;
        playEpisode(currentPlayingContext.showFolder, currentPlayingContext.seasonName, currentPlayingContext.fname, currentTime);
      }
    }

    function playEpisode(showFolder, seasonName, fname, seekTo = 0) {
      const decodedShow = decodeURIComponent(showFolder);
      const decodedSeason = decodeURIComponent(seasonName);
      const decodedFile = decodeURIComponent(fname);
      currentPlayingContext = { showFolder, seasonName, fname };
      
      const bufferSelect = document.getElementById('player-buffer-select');
      if (bufferSelect) bufferSelect.value = currentSelectedBufferMb;

      const streamUrl = `${window.location.origin}/api/stream?show=${encodeURIComponent(decodedShow)}&season=${encodeURIComponent(decodedSeason)}&file=${encodeURIComponent(decodedFile)}&buffer_mb=${currentSelectedBufferMb}`;
      const downloadUrl = `${window.location.origin}/api/download?show=${encodeURIComponent(decodedShow)}&season=${encodeURIComponent(decodedSeason)}&file=${encodeURIComponent(decodedFile)}`;
      activePlayingUrl = streamUrl;

      document.getElementById('player-title').innerHTML = `<span class="text-amber-400">▶️</span> ${decodedFile}`;
      document.getElementById('player-subtitle').innerText = `${decodedShow} • ${decodedSeason}`;
      
      const dlBtn = document.getElementById('player-download-btn');
      if (dlBtn) {
        dlBtn.href = downloadUrl;
        dlBtn.setAttribute('download', decodedFile);
      }

      document.getElementById('modal-video-player').classList.remove('hidden');

      // Initialize Artplayer Pro Web Player
      if (art) {
        art.destroy(false);
        art = null;
      }

      try {
        art = new Artplayer({
          container: '#artplayer-container',
          url: streamUrl,
          title: decodedFile,
          volume: 0.8,
          isLive: false,
          muted: false,
          autoplay: true,
          pip: true,
          autoSize: false,
          autoMini: true,
          screenshot: true,
          setting: true,
          loop: false,
          flip: true,
          playbackRate: true,
          aspectRatio: true,
          fullscreen: true,
          fullscreenWeb: true,
          subtitleOffset: true,
          miniProgressBar: true,
          mutex: true,
          backdrop: true,
          playsInline: true,
          autoPlayback: true,
          theme: '#e5a00d',
          icons: {
            loading: '<div class="text-xs font-semibold text-amber-400 animate-pulse">⚡ Đang nạp Cloud Stream...</div>'
          }
        });

        if (seekTo > 0) {
          art.on('ready', () => {
            art.currentTime = seekTo;
            art.play();
          });
        }
      } catch (e) {
        console.error("ArtPlayer init fallback:", e);
      }

      // Load external and muxed subtitles
      loadSubtitlesForVideo(decodedShow, decodedSeason, decodedFile);
    }

    function openInVLC() {
      if (!currentPlayingContext) return;
      const { showFolder, seasonName, fname } = currentPlayingContext;
      const directStreamUrl = `${window.location.origin}/api/stream?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
      const m3uUrl = `${window.location.origin}/api/playlist.m3u?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
      
      // 1. Try direct VLC URI scheme
      window.location.href = `vlc://${directStreamUrl}`;

      // 2. Also trigger M3U download fallback for 100% macOS VLC auto-launch
      setTimeout(() => {
        const a = document.createElement('a');
        a.href = m3uUrl;
        a.download = `${fname}.m3u`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
      }, 400);
    }

    function openInIINA() {
      if (!currentPlayingContext) return;
      const { showFolder, seasonName, fname } = currentPlayingContext;
      const directStreamUrl = `${window.location.origin}/api/stream?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
      const m3uUrl = `${window.location.origin}/api/playlist.m3u?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
      
      // 1. Try official IINA URI scheme for macOS
      window.location.href = `iina://open?url=${encodeURIComponent(directStreamUrl)}`;

      // 2. Also trigger M3U download fallback
      setTimeout(() => {
        const a = document.createElement('a');
        a.href = m3uUrl;
        a.download = `${fname}.m3u`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
      }, 400);
    }

    function downloadM3U() {
      if (!currentPlayingContext) return;
      const { showFolder, seasonName, fname } = currentPlayingContext;
      const m3uUrl = `${window.location.origin}/api/playlist.m3u?show=${encodeURIComponent(showFolder)}&season=${encodeURIComponent(seasonName)}&file=${encodeURIComponent(fname)}`;
      window.open(m3uUrl, '_blank');
    }

    async function loadSubtitlesForVideo(show, season, filename) {
      const select = document.getElementById('player-sub-select');
      if (!select) return;
      select.innerHTML = '<option value="none">⚪ Đang tìm phụ đề...</option>';
      
      try {
        const res = await fetch(`/api/subtitles?show=${encodeURIComponent(show)}&season=${encodeURIComponent(season)}&file=${encodeURIComponent(filename)}`);
        const data = await res.json();
        const subs = data.subtitles || [];
        
        let html = '<option value="none">⚪ Tắt Phụ Đề</option>';
        subs.forEach(s => {
          html += `<option value="${s.url}">${s.label}</option>`;
        });
        select.innerHTML = html;

        // Auto-select Vietnamese subtitle if available
        const viSub = subs.find(s => s.label.includes('Tiếng Việt') || s.label.includes('Vietsub') || s.label.includes('🇻🇳'));
        if (viSub) {
          select.value = viSub.url;
          changeSubtitle(viSub.url);
        } else if (subs.length > 0) {
          select.value = subs[0].url;
          changeSubtitle(subs[0].url);
        } else {
          changeSubtitle('none');
        }
      } catch (e) {
        select.innerHTML = '<option value="none">⚪ Không tìm thấy phụ đề</option>';
      }
    }

    function changeSubtitle(subUrl) {
      if (art) {
        if (subUrl && subUrl !== 'none') {
          art.subtitle.switch(subUrl, {
            name: 'Active Subtitle',
            default: true,
          });
          art.subtitle.show = true;
        } else {
          art.subtitle.show = false;
        }
      }
    }

    let isMiniPlayer = false;

    function toggleMiniPlayer() {
      const wrapper = document.getElementById('modal-video-player');
      const box = document.getElementById('player-modal-box');
      const miniBtn = document.getElementById('player-mini-btn');
      const bottomControls = document.getElementById('player-controls-bottom');
      const subtitle = document.getElementById('player-subtitle');

      isMiniPlayer = !isMiniPlayer;

      if (isMiniPlayer) {
        // Floating Mini Player docked at bottom-right
        wrapper.className = "fixed bottom-6 right-6 z-50 flex pointer-events-auto transition-all duration-300";
        box.className = "bg-zinc-950/95 backdrop-blur-md border border-amber-500/40 rounded-2xl w-80 sm:w-96 p-3 space-y-2 shadow-2xl flex flex-col transition-all duration-300";
        miniBtn.innerHTML = "🗖";
        miniBtn.title = "Phóng to toàn màn hình";
        if (bottomControls) bottomControls.classList.add('hidden');
        if (subtitle) subtitle.classList.add('hidden');
      } else {
        // Full Screen Player Page View
        wrapper.className = "fixed inset-0 z-50 bg-[#09090b] flex flex-col overflow-y-auto custom-scroll w-full h-full safe-top safe-bottom transition-all duration-300";
        box.className = "w-full flex-1 flex flex-col transition-all duration-300";
        miniBtn.innerHTML = "🗕";
        miniBtn.title = "Thu nhỏ góc màn hình";
        if (bottomControls) bottomControls.classList.remove('hidden');
        if (subtitle) subtitle.classList.remove('hidden');
      }
      
      if (art && art.resize) {
        setTimeout(() => { art.resize(); }, 150);
      }
    }

    function closeVideoPlayer() {
      if (art) {
        art.destroy(false);
        art = null;
      }
      isMiniPlayer = false;
      const wrapper = document.getElementById('modal-video-player');
      wrapper.className = "fixed inset-0 z-50 bg-[#09090b] hidden flex flex-col overflow-y-auto custom-scroll w-full h-full safe-top safe-bottom transition-all duration-300";
      const box = document.getElementById('player-modal-box');
      if (box) box.className = "w-full flex-1 flex flex-col transition-all duration-300";
      const miniBtn = document.getElementById('player-mini-btn');
      if (miniBtn) { miniBtn.innerHTML = "🗕"; miniBtn.title = "Thu nhỏ góc màn hình"; }
      const bottomControls = document.getElementById('player-controls-bottom');
      if (bottomControls) bottomControls.classList.remove('hidden');
      const subtitle = document.getElementById('player-subtitle');
      if (subtitle) subtitle.classList.remove('hidden');
    }

    function copyActiveStreamUrl() {
      if (activePlayingUrl) {
        navigator.clipboard.writeText(activePlayingUrl);
        showToast('📋 Đã copy đường link Stream trực tiếp vào bộ nhớ tạm!', 'success');
      }
    }

    function copyEpisodeLink(showFolder, seasonName, fname) {
      const decodedShow = decodeURIComponent(showFolder);
      const decodedSeason = decodeURIComponent(seasonName);
      const decodedFile = decodeURIComponent(fname);
      const directUrl = `${window.location.origin}/api/stream?show=${encodeURIComponent(decodedShow)}&season=${encodeURIComponent(decodedSeason)}&file=${encodeURIComponent(decodedFile)}`;
      
      navigator.clipboard.writeText(directUrl);
      showToast(`📋 Đã copy link xem/tải tập phim:\n${decodedFile}`, 'success');
    }


export {
  art,
  activePlayingUrl,
  currentPlayingContext,
  currentSelectedBufferMb,
  isMiniPlayer,
  changePlayerBuffer,
  playEpisode,
  openInVLC,
  openInIINA,
  downloadM3U,
  loadSubtitlesForVideo,
  changeSubtitle,
  toggleMiniPlayer,
  closeVideoPlayer,
  copyActiveStreamUrl,
  copyEpisodeLink
};
