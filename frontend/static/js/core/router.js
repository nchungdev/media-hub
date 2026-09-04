/**
 * SPA Router & Tab Navigation Controller
 */

export function getInitialTab() {
  // 1. Check URL pathname (e.g. /subtitles, /torbox, /collection)
  const cleanPath = (typeof window !== 'undefined' && window.location.pathname)
    ? window.location.pathname.replace(/^\/+|\/+$/g, '').toLowerCase()
    : '';

  // 2. Check URL search query (?tab=subtitles)
  const queryTab = (typeof window !== 'undefined')
    ? new URLSearchParams(window.location.search).get('tab')
    : null;

  // 3. Check URL hash (#subtitles)
  const hashTab = (typeof window !== 'undefined' && window.location.hash)
    ? window.location.hash.replace(/^#\/?/, '').toLowerCase()
    : '';

  // 4. Check localStorage
  let savedTab = null;
  try {
    savedTab = localStorage.getItem('media_hub_active_tab');
  } catch (_) {}

  const candidate = cleanPath || queryTab || hashTab || savedTab || 'home';
  const tabAliases = {
    'overview': 'home',
    'downloads': 'torbox',
    'torrents': 'torbox',
    'downloader': 'torbox',
    'collection': 'collection',
    'collections': 'collection',
    'plex': 'collection',
    'library': 'collection',
    'gdrive': 'collection',
    'sync': 'collection',
    'pipelines': 'collection',
    'subtitle-studio': 'subtitles',
    'tokens': 'tokens',
    'token-usage': 'tokens',
    'analytics': 'tokens',
    'console': 'console',
    'logs': 'console',
    'terminal': 'console',
    'cli': 'console',
    'chat': 'agent',
    'config': 'settings'
  };
  const normalized = tabAliases[candidate] || candidate;
  const validTabs = ['home', 'torbox', 'collection', 'subtitles', 'tokens', 'console', 'settings', 'agent'];
  return validTabs.includes(normalized) ? normalized : 'home';
}

export function setTab(tab, updateUrl = true) {
  window.scrollTo({ top: 0, behavior: 'instant' });

  const tabAliases = {
    'overview': 'home',
    'downloads': 'torbox',
    'torrents': 'torbox',
    'downloader': 'torbox',
    'collection': 'collection',
    'collections': 'collection',
    'plex': 'collection',
    'library': 'collection',
    'gdrive': 'collection',
    'sync': 'collection',
    'pipelines': 'collection',
    'subtitle-studio': 'subtitles',
    'tokens': 'tokens',
    'token-usage': 'tokens',
    'analytics': 'tokens',
    'console': 'console',
    'logs': 'console',
    'terminal': 'console',
    'cli': 'console',
    'chat': 'agent',
    'config': 'settings'
  };
  tab = tabAliases[tab] || tab;
  const tabs = ['home', 'torbox', 'collection', 'subtitles', 'tokens', 'console', 'settings', 'agent'];
  if (!tabs.includes(tab)) tab = 'home';

  // Persist active tab selection to localStorage
  try {
    localStorage.setItem('media_hub_active_tab', tab);
  } catch (_) {}

  // Synchronize URL path on the browser address bar
  if (updateUrl && typeof window !== 'undefined' && window.history) {
    const targetPath = (tab === 'home') ? '/' : '/' + tab;
    if (window.location.pathname !== targetPath) {
      window.history.pushState({ tab }, '', targetPath);
    }
  }

  tabs.forEach(t => {
    const panel = document.getElementById(`tab-${t}`);
    const btn = document.getElementById(`tab-btn-${t}`);
    const label = document.getElementById(`tab-label-${t}`);
    const dot = document.getElementById(`tab-dot-${t}`);
    const mbtn = document.getElementById(`mobile-tab-btn-${t}`);

    if (panel) {
      panel.classList.add('hidden');
      panel.style.display = 'none';
    }
    if (btn) {
      btn.className = "w-full px-3 py-2.5 rounded-xl flex items-center justify-between transition cursor-pointer text-zinc-400 hover:text-white hover:bg-zinc-900 group";
    }
    if (label) {
      label.className = "text-xs text-zinc-300 group-hover:text-white font-bold truncate";
    }
    if (dot) {
      if (t === 'agent') {
        dot.className = "w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse shrink-0";
      } else if (t === 'subtitles' || t === 'tokens' || t === 'collection') {
        dot.className = "w-1.5 h-1.5 rounded-full bg-cyan-400 hidden shrink-0";
      } else {
        dot.classList.add('hidden');
      }
    }
    if (mbtn) {
      mbtn.classList.remove('text-blue-400', 'font-bold');
      mbtn.classList.add('text-zinc-400');
    }
  });

  const activePanel = document.getElementById(`tab-${tab}`);
  const activeBtn = document.getElementById(`tab-btn-${tab}`);
  const activeLabel = document.getElementById(`tab-label-${tab}`);
  const activeDot = document.getElementById(`tab-dot-${tab}`);
  const activeMBtn = document.getElementById(`mobile-tab-btn-${tab}`);

  if (activePanel) {
    activePanel.classList.remove('hidden');
    activePanel.style.display = (tab === 'agent') ? 'flex' : 'block';
  }

  if (activeBtn) {
    activeBtn.className = "w-full px-3 py-2.5 rounded-xl flex items-center justify-between transition cursor-pointer bg-blue-600/10 text-blue-400 border border-blue-500/20 font-bold shadow-sm group";
  }
  if (activeLabel) {
    activeLabel.className = "text-xs text-blue-400 font-bold truncate";
  }
  if (activeDot) {
    activeDot.classList.remove('hidden');
    if (tab !== 'agent') {
      activeDot.className = "w-1.5 h-1.5 rounded-full bg-blue-400 shrink-0";
    }
  }
  if (activeMBtn) {
    activeMBtn.classList.remove('text-zinc-400');
    activeMBtn.classList.add('text-blue-400', 'font-bold');
  }

  // Strictly control agent input dock visibility
  const agentDock = document.getElementById('agent-input-dock');
  if (agentDock) {
    agentDock.style.display = (tab === 'agent') ? 'block' : 'none';
  }

  // Tab dynamic loaders
  if (tab === 'home') {
    if (typeof window.fetchDashboardOverview === 'function') window.fetchDashboardOverview();
    if (typeof window.fetchData === 'function') window.fetchData();
  } else if (tab === 'collection') {
    if (typeof window.loadMediaCollections === 'function') window.loadMediaCollections();
  } else if (tab === 'torbox') {
    if (typeof window.fetchTorrents === 'function') window.fetchTorrents();
  } else if (tab === 'subtitles') {
    if (typeof window.loadSubtitleStudioData === 'function') window.loadSubtitleStudioData();
    if (typeof window.loadSubtitlesStaging === 'function') window.loadSubtitlesStaging();
  } else if (tab === 'tokens') {
    if (typeof window.loadTokenUsageReport === 'function') window.loadTokenUsageReport();
  } else if (tab === 'console') {
    if (typeof window.pollTabConsoleLogs === 'function') window.pollTabConsoleLogs(true);
    if (window.tabConsoleInterval) clearInterval(window.tabConsoleInterval);
    window.tabConsoleInterval = setInterval(() => {
      if (typeof window.pollTabConsoleLogs === 'function') window.pollTabConsoleLogs();
    }, 1200);
  } else if (tab === 'settings') {
    if (typeof window.fetchSettings === 'function') window.fetchSettings();
    if (typeof window.checkAllServicesStatus === 'function') window.checkAllServicesStatus();
  } else if (tab === 'agent') {
    if (typeof window.loadAgentQueueFull === 'function') window.loadAgentQueueFull();
    setTimeout(() => {
      const inp = document.getElementById('full-agent-input');
      if (inp) inp.focus();
    }, 150);
  }

  if (tab !== 'console' && window.tabConsoleInterval) {
    clearInterval(window.tabConsoleInterval);
    window.tabConsoleInterval = null;
  }
}

export function initRouter() {
  window.addEventListener('popstate', (e) => {
    if (e.state && e.state.tab) {
      setTab(e.state.tab, false);
    } else {
      setTab(getInitialTab(), false);
    }
  });
}

window.setTab = setTab;
window.getInitialTab = getInitialTab;
