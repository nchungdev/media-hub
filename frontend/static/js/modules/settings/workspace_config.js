/**
 * Workspace Directory & System Settings Storage
 */
import { showToast } from '../../core/toast.js';

    async function ensureCliService() {
      try {
        const res = await fetch('/api/agent/service/ensure', { method: 'POST' });
        const data = await res.json();
        if (data.status === 'attached' || data.is_running) {
          console.log(`[CLI Service] Attached to active background CLI [${data.active_job?.cli || 'CLI'}]`);
          pollTabConsoleLogs(true);
        }
      } catch (e) {}
    }


    
    async function startAria2Daemon() {
      showToast('⏳ Đang khởi chạy tiến trình Aria2c RPC Daemon...', 'info', 2000);
      try {
        const res = await fetch('/api/aria2/control', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ operation: 'start' })
        });
        const data = await res.json();
        if (data.success) {
          showToast('✅ ' + data.message, 'success');
          setTimeout(checkAllServicesStatus, 600);
        } else {
          showToast('❌ ' + (data.error || 'Lỗi khởi chạy'), 'error');
        }
      } catch (e) {
        showToast('❌ Lỗi kết nối: ' + e, 'error');
      }
    }

    async function checkAllServicesStatus() {
      try {
        const res = await fetch('/api/services/status');
        const data = await res.json();
        if (!data.success || !data.services) return;
        const s = data.services;

        const renderBadge = (elementId, serviceInfo, connectedText) => {
          const el = document.getElementById(elementId);
          if (!el) return;
          if (serviceInfo && serviceInfo.connected) {
            el.className = 'px-2.5 py-1 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 flex items-center gap-1.5 shadow-sm';
            el.innerHTML = `<span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span> ${connectedText || 'Đang kết nối'}`;
            el.title = serviceInfo.detail || 'Kết nối thành công';
          } else {
            el.className = 'px-2.5 py-1 rounded-full text-[10px] font-semibold bg-red-500/10 text-red-400 border border-red-500/30 flex items-center gap-1.5 shadow-sm';
            el.innerHTML = `<span class="w-1.5 h-1.5 rounded-full bg-red-500"></span> Mất kết nối`;
            el.title = (serviceInfo && serviceInfo.detail) || 'Không thể kết nối';
          }
        };

        renderBadge('status-badge-gdrive', s.gdrive, 'Đang kết nối');
        renderBadge('status-badge-nas', s.nas, 'Đang kết nối');
        renderBadge('status-badge-torbox', s.torbox, 'Đang kết nối');
        renderBadge('status-badge-tmdb', s.tmdb, 'Đang kết nối');
        renderBadge('status-badge-aria2', s.aria2, 'Đang kết nối');
        renderBadge('status-badge-ytdlp', s.ytdlp, 'Sẵn sàng');
        renderBadge('status-badge-direct', s.direct, 'Sẵn sàng');
      } catch (e) {
        console.error("Error checking service statuses:", e);
      }
    }

    window.currentWorkspaceDir = '';

    async function chooseNativeDirectory(targetInputId) {
      try {
        const res = await fetch('/api/fs/choose_directory', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: '{}'
        });
        const data = await res.json();
        if (data.success && data.path) {
          if (targetInputId) {
            const el = document.getElementById(targetInputId);
            if (el) el.value = data.path;
          }
          return data.path;
        }
      } catch (e) {
        console.error('Error choosing directory:', e);
      }
      return null;
    }

    function updateSidebarWorkspaceUI(wsPath) {
      if (!wsPath) return;
      window.currentWorkspaceDir = wsPath;
      const cleanPath = wsPath.replace(/\/+$/, '');
      const parts = cleanPath.split('/').filter(Boolean);
      const baseName = parts[parts.length - 1] || cleanPath;
      const nameEl = document.getElementById('sidebar-workspace-name');
      const pathEl = document.getElementById('sidebar-workspace-path');
      if (nameEl) nameEl.textContent = baseName;
      if (pathEl) {
        pathEl.textContent = cleanPath;
        pathEl.title = cleanPath;
      }
    }

    function openWorkspaceSetupModal() {
      const currentWs = window.currentWorkspaceDir || '/Volumes/512GB/AI Workspace';
      const inp = document.getElementById('ws-modal-input');
      if (inp) inp.value = currentWs;
      openModal('modal-workspace-setup');
    }

    function setWorkspaceInputPath(p) {
      const inp = document.getElementById('ws-modal-input');
      if (inp) inp.value = p;
    }

    async function applyWorkspaceSetup() {
      const inp = document.getElementById('ws-modal-input');
      const path = inp ? inp.value.trim() : '';
      if (!path) {
        alert('Vui lòng nhập hoặc chọn một thư mục làm việc hợp lệ.');
        return;
      }
      const btn = document.getElementById('btn-apply-ws');
      if (btn) {
        btn.disabled = true;
        btn.innerHTML = '<span class="animate-spin">⚙️</span> Đang thiết lập & quét...';
      }

      try {
        const res = await fetch('/api/workspace/set', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ workspace_dir: path })
        });
        const data = await res.json();
        if (data.success) {
          window.currentWorkspaceDir = data.workspace_dir;
          updateSidebarWorkspaceUI(data.workspace_dir);
          closeModal('modal-workspace-setup');
          showToast(`📁 Đã thiết lập thư mục làm việc: ${data.workspace_dir}`, 'success');
          // Refresh library and settings
          await fetchSettings();
          if (typeof loadSubtitleStudioData === 'function') {
            await loadSubtitleStudioData();
          }
        } else {
          alert(`❌ Lỗi: ${data.error || 'Không thể thiết lập thư mục'}`);
        }
      } catch (e) {
        alert(`❌ Lỗi kết nối: ${e}`);
      } finally {
        if (btn) {
          btn.disabled = false;
          btn.innerHTML = '<span>🚀</span> Áp Dụng & Quét Dữ Liệu';
        }
      }
    }

    async function fetchSettings() {
      try {
        const res = await fetch('/api/settings');
        const cfg = await res.json();
        
        const wsDir = cfg.workspace_dir || cfg.media_hub_home || '';
        updateSidebarWorkspaceUI(wsDir);

        if (document.getElementById('cfg-workspace-dir')) document.getElementById('cfg-workspace-dir').value = wsDir;
        if (document.getElementById('cfg-default-provider')) document.getElementById('cfg-default-provider').value = cfg.default_provider || 'torbox';
        if (document.getElementById('cfg-max-downloads')) document.getElementById('cfg-max-downloads').value = cfg.max_concurrent_downloads || 2;
        if (document.getElementById('cfg-staging-dir')) document.getElementById('cfg-staging-dir').value = cfg.staging_dir || '';
        if (document.getElementById('cfg-hub-home')) document.getElementById('cfg-hub-home').value = cfg.media_hub_home || '';
        if (document.getElementById('cfg-movies-dir')) document.getElementById('cfg-movies-dir').value = cfg.movies_dirname || '';
        if (document.getElementById('cfg-tv-dir')) document.getElementById('cfg-tv-dir').value = cfg.tv_dirname || '';
        if (document.getElementById('cfg-logs-dir')) document.getElementById('cfg-logs-dir').value = cfg.logs_dir || '';
        if (document.getElementById('cfg-torbox-token')) document.getElementById('cfg-torbox-token').value = cfg.torbox_token || '';
        if (document.getElementById('cfg-tmdb-key')) document.getElementById('cfg-tmdb-key').value = cfg.tmdb_api_key || '';
        if (document.getElementById('cfg-tmdb-lang')) document.getElementById('cfg-tmdb-lang').value = cfg.tmdb_lang || 'vi-VN';
        if (document.getElementById('cfg-aria2-host')) document.getElementById('cfg-aria2-host').value = cfg.aria2_rpc_host || '127.0.0.1';
        if (document.getElementById('cfg-aria2-port')) document.getElementById('cfg-aria2-port').value = cfg.aria2_rpc_port || 6800;
        if (document.getElementById('cfg-aria2-secret')) document.getElementById('cfg-aria2-secret').value = cfg.aria2_rpc_secret || '';
        
        if (document.getElementById('cfg-nas-host')) document.getElementById('cfg-nas-host').value = cfg.nas_host || '';
        if (document.getElementById('cfg-nas-user')) document.getElementById('cfg-nas-user').value = cfg.nas_user || 'admin';
        if (document.getElementById('cfg-nas-port')) document.getElementById('cfg-nas-port').value = cfg.nas_port || 22;
        if (document.getElementById('cfg-nas-ssh-key')) document.getElementById('cfg-nas-ssh-key').value = cfg.nas_ssh_key || '';
        if (document.getElementById('cfg-nas-path')) document.getElementById('cfg-nas-path').value = cfg.nas_path || '/volume1/video/TV Shows';

        if (document.getElementById('cfg-gdrive-remote')) document.getElementById('cfg-gdrive-remote').value = cfg.gdrive_remote || 'gdrive';
        if (document.getElementById('cfg-gdrive-root')) document.getElementById('cfg-gdrive-root').value = cfg.gdrive_root || 'Phim';
        if (document.getElementById('cfg-sync-transfers')) document.getElementById('cfg-sync-transfers').value = cfg.sync_transfers || 4;

        const targets = cfg.sync_targets || ['drive'];
        if (document.getElementById('cfg-target-drive')) document.getElementById('cfg-target-drive').checked = targets.includes('drive');
        if (document.getElementById('cfg-target-nas')) document.getElementById('cfg-target-nas').checked = targets.includes('nas');
        if (document.getElementById('cfg-auto-purge')) document.getElementById('cfg-auto-purge').checked = cfg.auto_purge !== false;
        window.currentAgyProfile = cfg.agy_cli_profile || 'auto';
        if (document.getElementById('cfg-agy-profile')) document.getElementById('cfg-agy-profile').value = window.currentAgyProfile;

        window.currentCloudflareToken = cfg.cloudflare_tunnel_token || '';
        window.currentCloudflareHostname = cfg.cloudflare_tunnel_hostname || '';
        if (document.getElementById('cfg-cloudflare-tunnel-token')) {
          document.getElementById('cfg-cloudflare-tunnel-token').value = window.currentCloudflareToken;
        }
        if (document.getElementById('cfg-cloudflare-tunnel-hostname')) {
          document.getElementById('cfg-cloudflare-tunnel-hostname').value = window.currentCloudflareHostname;
        }

        // Update NAS glance card in library view
        const nasPathEl = document.getElementById('nas-path-display');
        if (nasPathEl) nasPathEl.textContent = '📁 ' + (cfg.nas_path || '/volume1/video/TV Shows');
        
        const host = cfg.nas_host || '192.168.1.50';
        const nasPlexEl = document.getElementById('nas-link-plex');
        if (nasPlexEl) nasPlexEl.href = `http://${host}:32400/web`;
        
        const nasDsmEl = document.getElementById('nas-link-dsm');
        if (nasDsmEl) nasDsmEl.href = `http://${host}:5000`;
      } catch (e) {
        console.error("Error fetching settings:", e);
      }
    }

    // Switch the active Antigravity CLI instance immediately, without waiting for "Lưu Cấu Hình".
    // agent_bridge re-reads agy_cli_profile on every dispatch, so persisting it is enough for the
    // next command to land on the newly selected instance.
    async function switchAgyProfile(profile) {
      const select = document.getElementById('cfg-agy-profile');
      const labels = {
        auto: '🔄 Tự Động (agy2 → agy)',
        agy2: '⚡ agy2 (Secondary)',
        agy: '🛡️ agy (Primary)'
      };
      const prev = window.currentAgyProfile || 'auto';
      if (profile === prev) return;
      if (select) select.disabled = true;

      try {
        // Read-modify-write: /api/settings replaces the whole config, so never POST a partial object.
        const cur = await fetch('/api/settings').then(r => {
          if (!r.ok) throw new Error(`HTTP ${r.status}`);
          return r.json();
        });
        cur.agy_cli_profile = profile;

        const res = await fetch('/api/settings', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify(cur)
        });
        const data = await res.json();
        if (!res.ok || !data.success) throw new Error(data.error || `HTTP ${res.status}`);

        window.currentAgyProfile = profile;
        showToast(`⚡ Đã chuyển sang ${labels[profile] || profile}. Lệnh kế tiếp sẽ chạy trên instance này.`, 'success', 3000);

        // Refresh the CLI service badge so the console reflects the new instance right away
        if (typeof window.checkAllServicesStatus === 'function') window.checkAllServicesStatus();
        if (typeof window.updateCliToggleBtn === 'function') window.updateCliToggleBtn();
      } catch (e) {
        if (select) select.value = prev;
        showToast(`❌ Không đổi được CLI instance: ${e.message}`, 'error', 4000);
      } finally {
        if (select) select.disabled = false;
      }
    }

    async function saveSettings() {
      const targets = [];
      if (document.getElementById('cfg-target-drive')?.checked) targets.push('drive');
      if (document.getElementById('cfg-target-nas')?.checked) targets.push('nas');

      const wsDir = document.getElementById('cfg-workspace-dir')?.value?.trim() || '';

      const payload = {
        workspace_dir: wsDir,
        default_provider: document.getElementById('cfg-default-provider')?.value || 'torbox',
        max_concurrent_downloads: parseInt(document.getElementById('cfg-max-downloads')?.value || 2),
        staging_dir: document.getElementById('cfg-staging-dir')?.value || '',
        media_hub_home: wsDir ? (wsDir.endsWith('.media-hub') ? wsDir : wsDir + '/.media-hub') : '',
        movies_dirname: document.getElementById('cfg-movies-dir')?.value || '',
        tv_dirname: document.getElementById('cfg-tv-dir')?.value || '',
        logs_dir: document.getElementById('cfg-logs-dir')?.value || '',
        torbox_token: document.getElementById('cfg-torbox-token')?.value || '',
        tmdb_api_key: document.getElementById('cfg-tmdb-key')?.value || '',
        tmdb_lang: document.getElementById('cfg-tmdb-lang')?.value || 'vi-VN',
        aria2_rpc_host: document.getElementById('cfg-aria2-host')?.value || '127.0.0.1',
        aria2_rpc_port: parseInt(document.getElementById('cfg-aria2-port')?.value || 6800),
        aria2_rpc_secret: document.getElementById('cfg-aria2-secret')?.value || '',
        nas_host: document.getElementById('cfg-nas-host')?.value || '',
        nas_user: document.getElementById('cfg-nas-user')?.value || 'admin',
        nas_port: parseInt(document.getElementById('cfg-nas-port')?.value || 22),
        nas_ssh_key: document.getElementById('cfg-nas-ssh-key')?.value || '',
        nas_path: document.getElementById('cfg-nas-path')?.value || '/volume1/video/TV Shows',
        gdrive_remote: document.getElementById('cfg-gdrive-remote')?.value || 'gdrive',
        gdrive_root: document.getElementById('cfg-gdrive-root')?.value || 'Phim/TV Shows',
        sync_transfers: parseInt(document.getElementById('cfg-sync-transfers')?.value || 4),
        sync_targets: targets,
        auto_purge: !!document.getElementById('cfg-auto-purge')?.checked,
        agy_cli_profile: document.getElementById('cfg-agy-profile')?.value || 'auto',
        cloudflare_tunnel_token: document.getElementById('cfg-cloudflare-tunnel-token')?.value !== undefined 
          ? document.getElementById('cfg-cloudflare-tunnel-token').value.trim() 
          : (window.currentCloudflareToken || ''),
        cloudflare_tunnel_hostname: document.getElementById('cfg-cloudflare-tunnel-hostname')?.value !== undefined 
          ? document.getElementById('cfg-cloudflare-tunnel-hostname').value.trim() 
          : (window.currentCloudflareHostname || '')
      };

      try {
        const res = await fetch('/api/settings', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (data.success) {
          // Also save Quota Guard configuration if customized
          const qDaily = parseInt(document.getElementById('cfg-quota-daily')?.value || 30);
          const qWeekly = parseInt(document.getElementById('cfg-quota-weekly')?.value || 150);
          try {
            await fetch('/api/agent/quota_config', {
              method: 'POST',
              headers: {'Content-Type': 'application/json'},
              body: JSON.stringify({ daily_limit: qDaily, weekly_limit: qWeekly })
            });
            loadQuotaGuardStatus();
          } catch (err) {}

          if (wsDir) updateSidebarWorkspaceUI(wsDir);
          showToast('💾 ' + (data.message || 'Đã lưu cấu hình thành công!'), 'success');
        } else {
          showToast('Lỗi: ' + (data.error || 'Không thể lưu'), 'error');
        }
      } catch (e) {
        showToast('Lỗi kết nối: ' + e, 'error');
      }
    }

    /* ==================== CLOUDFLARE QUICK TUNNEL (TRYCLOUDFLARE) ==================== */

    function openModal(id) { 
      const el = document.getElementById(id);
      if (!el) return;
      el.classList.remove('hidden');
      el.classList.add('flex');
    }
    function closeModal(id) { 
      const el = document.getElementById(id);
      if (!el) return;
      el.classList.add('hidden');
      el.classList.remove('flex');
    }



export {
  switchAgyProfile,
  ensureCliService,
  startAria2Daemon,
  checkAllServicesStatus,
  chooseNativeDirectory,
  updateSidebarWorkspaceUI,
  openWorkspaceSetupModal,
  setWorkspaceInputPath,
  applyWorkspaceSetup,
  fetchSettings,
  saveSettings,
  openModal,
  closeModal
};
