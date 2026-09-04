/**
 * TryCloudflare Quick Remote Tunnel Controller
 */
import { showToast } from '../../core/toast.js';

    async function fetchTunnelStatus() {
      try {
        const res = await fetch('/api/tunnel/status');
        const st = await res.json();
        updateTunnelUI(st);
      } catch (e) {}
    }

    function updateTunnelUI(st) {
      const isRunning = !!st.running;
      const url = st.url || '';

      // Sidebar indicator
      const sideBadge = document.getElementById('sidebar-tunnel-badge');
      if (sideBadge) {
        if (isRunning && url) {
          sideBadge.className = "cursor-pointer font-mono text-[9px] text-emerald-400 font-bold hover:underline transition flex items-center gap-1";
          sideBadge.innerHTML = `<span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span> Public`;
          sideBadge.title = `Đang mở: ${url} (Bấm để copy)`;
          sideBadge.onclick = () => {
            navigator.clipboard.writeText(url);
            showToast('📋 Đã sao chép URL Remote!', 'success', 1800);
          };
        } else {
          sideBadge.className = "cursor-pointer font-mono text-[9px] text-zinc-500 hover:text-amber-400 transition";
          sideBadge.innerHTML = "Offline";
          sideBadge.title = "Bấm để vào cài đặt bật Tunnel";
          sideBadge.onclick = () => setTab('settings');
        }
      }

      // Settings Tab UI Elements
      const badgeStatus = document.getElementById('tunnel-badge-status');
      const activeBox = document.getElementById('tunnel-active-box');
      const publicUrlInput = document.getElementById('tunnel-public-url');
      const btnToggle = document.getElementById('btn-tunnel-toggle');
      const btnIcon = document.getElementById('tunnel-btn-icon');
      const btnText = document.getElementById('tunnel-btn-text');
      const binPathEl = document.getElementById('tunnel-binary-path');
      const startTimeEl = document.getElementById('tunnel-start-time');

      if (binPathEl) {
        binPathEl.textContent = st.binary || (st.installed ? 'cloudflared' : 'Chưa cài đặt (brew install cloudflared)');
      }

      if (isRunning && url) {
        if (badgeStatus) {
          badgeStatus.className = "px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 text-[10px] border border-emerald-500/20 font-mono font-bold flex items-center gap-1";
          badgeStatus.innerHTML = '<span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span> 🟢 Đang Bật (Public)';
        }
        if (activeBox) activeBox.classList.remove('hidden');
        if (publicUrlInput) publicUrlInput.value = url;
        if (startTimeEl) startTimeEl.textContent = 'Khởi động: ' + (st.started_at || '--');

        if (btnToggle) {
          btnToggle.className = "px-4 py-2 bg-red-600/20 hover:bg-red-600/30 border border-red-500/30 text-red-300 hover:text-white rounded-xl text-xs font-bold transition flex items-center gap-1.5 cursor-pointer";
          if (btnIcon) btnIcon.textContent = "⏹";
          if (btnText) btnText.textContent = "Tắt Truy Cập Từ Xa";
        }
      } else {
        if (badgeStatus) {
          badgeStatus.className = "px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-400 text-[10px] border border-zinc-700 font-mono font-bold";
          badgeStatus.innerHTML = "⚪ Đang Tắt (Local Only)";
        }
        if (activeBox) activeBox.classList.add('hidden');
        if (publicUrlInput) publicUrlInput.value = '';

        if (btnToggle) {
          btnToggle.className = "px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded-xl text-xs font-bold transition flex items-center gap-1.5 shadow-md shadow-amber-600/20 cursor-pointer";
          if (btnIcon) btnIcon.textContent = "⚡";
          if (btnText) btnText.textContent = "Bật Truy Cập Từ Xa";
        }
      }
    }

    async function toggleCloudflareTunnel() {
      const btn = document.getElementById('btn-tunnel-toggle');
      const isCurrentlyRunning = document.getElementById('tunnel-btn-text')?.textContent === 'Tắt Truy Cập Từ Xa';

      if (btn) btn.disabled = true;

      try {
        if (isCurrentlyRunning) {
          showToast('⏳ Đang dừng Cloudflare Tunnel...', 'info', 2000);
          const res = await fetch('/api/tunnel/stop', {method: 'POST'});
          const data = await res.json();
          showToast(data.message || 'Đã tắt Tunnel', 'info');
          fetchTunnelStatus();
        } else {
          showToast('🚀 Đang yêu cầu Cloudflare Edge cấp phát URL...', 'info', 4000);
          const res = await fetch('/api/tunnel/start', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({port: 8888})
          });
          const data = await res.json();
          if (data.success) {
            showToast(`🎉 Tunnel đã mở: ${data.url}`, 'success', 5000);
            fetchTunnelStatus();
          } else {
            showToast(`❌ ${data.error || 'Không thể tạo tunnel'}`, 'error', 5000);
          }
        }
      } catch (e) {
        showToast(`❌ Lỗi: ${e.message || e}`, 'error');
      } finally {
        if (btn) btn.disabled = false;
      }
    }

    function copyTunnelUrl() {
      const input = document.getElementById('tunnel-public-url');
      if (input && input.value) {
        navigator.clipboard.writeText(input.value);
        showToast('📋 Đã sao chép URL Remote vào bộ nhớ đệm!', 'success', 2500);
      }
    }

    function openTunnelUrl() {
      const input = document.getElementById('tunnel-public-url');
      if (input && input.value) {
        window.open(input.value, '_blank');
      }
    }


export {
  fetchTunnelStatus,
  updateTunnelUI,
  toggleCloudflareTunnel,
  copyTunnelUrl,
  openTunnelUrl
};
