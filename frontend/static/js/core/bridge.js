/**
 * Desktop Native App Bridge (Adapter Pattern for Tauri 2 & Electron)
 */
export const desktopBridge = {
  isTauri: !!(window.__TAURI__ && window.__TAURI__.core),
  isElectron: !!(window.electronAPI && window.electronAPI.isElectron),
  get isDesktop() {
    return this.isTauri || this.isElectron;
  },
  async init() {
    if (!this.isDesktop) return;
    try {
      const row = document.getElementById('sidebar-desktop-row');
      const badge = document.getElementById('sidebar-desktop-badge');
      if (row) row.style.display = 'flex';

      if (this.isTauri) {
        const info = await window.__TAURI__.core.invoke('get_app_info');
        if (badge && info) {
          badge.innerText = `v${info.version} (🦀 Rust)`;
          badge.className = "font-mono text-[9px] text-orange-400 font-bold";
        }
      } else if (this.isElectron) {
        const infoRes = await window.electronAPI.app.getInfo();
        if (infoRes && infoRes.success && badge) {
          badge.innerText = `v${infoRes.data.version} (Electron)`;
        }
      }
    } catch (_) {}
  },
  async openExternal(url) {
    if (this.isTauri) {
      try {
        return await window.__TAURI__.core.invoke('open_external', { url });
      } catch (_) {}
    } else if (this.isElectron && window.electronAPI.system) {
      return await window.electronAPI.system.openExternal(url);
    }
    window.open(url, '_blank');
  },
  async writeClipboard(text) {
    if (this.isTauri) {
      if (navigator.clipboard) await navigator.clipboard.writeText(text);
      return;
    } else if (this.isElectron && window.electronAPI.system) {
      return await window.electronAPI.system.writeClipboard(text);
    }
    if (navigator.clipboard) {
      await navigator.clipboard.writeText(text);
    }
  },
  async showNotification(title, body) {
    if (this.isElectron && window.electronAPI.system) {
      return await window.electronAPI.system.showNotification({ title, body });
    }
    if (window.Notification && Notification.permission === 'granted') {
      new Notification(title, { body });
    }
  }
};

window.desktopBridge = desktopBridge;
