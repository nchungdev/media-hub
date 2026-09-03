/**
 * @file index.js
 * @description Preload Script Entrypoint.
 * Employs the Bridge Pattern with Electron's contextBridge to expose safe, typed APIs to the Renderer window.
 * Strictly adheres to Context Isolation (nodeIntegration: false, contextIsolation: true).
 */

'use strict';

const { contextBridge, ipcRenderer } = require('electron');
const { createAppApi } = require('./apis/appApi');
const { createSystemApi } = require('./apis/systemApi');
const { createServerApi } = require('./apis/serverApi');

// Compose exposed API object
const electronAPI = Object.freeze({
  isElectron: true,
  app: createAppApi(ipcRenderer),
  system: createSystemApi(ipcRenderer),
  server: createServerApi(ipcRenderer)
});

// Expose safely to the renderer execution context
contextBridge.exposeInMainWorld('electronAPI', electronAPI);

console.log('[Preload] 🛡️ ContextBridge initialized. window.electronAPI exposed safely.');
