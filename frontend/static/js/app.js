/**
 * Antigravity Media Hub - Main Application Bootstrap
 * Modular Architecture Entry Point
 */

import { desktopBridge } from './core/bridge.js';
import { showToast, dismissToast } from './core/toast.js';
import { apiFetch } from './core/api.js';
import { getInitialTab, setTab, initRouter } from './core/router.js';

// Feature Modules
import './modules/overview.js';
import './modules/torbox.js';
import './modules/collections.js';
import './modules/subtitles.js';
import './modules/settings.js';
import './modules/agent.js';
import './modules/services.js';

// Setup Native Bridge
document.addEventListener('DOMContentLoaded', () => {
  desktopBridge.init();
});

// Setup SPA Navigation History
initRouter();

// Application Bootstrap
const initialActiveTab = getInitialTab();
setTab(initialActiveTab, true);

if (typeof window.fetchData === 'function') window.fetchData();
if (typeof window.fetchSettings === 'function') window.fetchSettings();
if (typeof window.fetchTunnelStatus === 'function') window.fetchTunnelStatus();
if (typeof window.loadQuotaGuardStatus === 'function') window.loadQuotaGuardStatus();
if (typeof window.ensureCliService === 'function') window.ensureCliService();

// Polling Daemons
setInterval(() => {
  if (typeof window.fetchData === 'function') window.fetchData();
}, 4000);

setInterval(() => {
  if (typeof window.fetchTunnelStatus === 'function') window.fetchTunnelStatus();
}, 8000);

setInterval(() => {
  if (typeof window.loadQuotaGuardStatus === 'function') window.loadQuotaGuardStatus();
}, 10000);

console.log('🚀 [MediaHub] Frontend Modular Architecture initialized successfully.');
