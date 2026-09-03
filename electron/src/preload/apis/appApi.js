/**
 * @file appApi.js
 * @description Preload API wrapper for Application and Window operations.
 */

'use strict';

const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');

/**
 * Creates the App API bridge.
 * @param {Electron.IpcRenderer} ipcRenderer
 */
function createAppApi(ipcRenderer) {
  return Object.freeze({
    getInfo: () => ipcRenderer.invoke(IPC_CHANNELS.APP.GET_INFO),
    getPlatform: () => ipcRenderer.invoke(IPC_CHANNELS.APP.GET_PLATFORM),
    minimize: () => ipcRenderer.invoke(IPC_CHANNELS.APP.MINIMIZE),
    maximize: () => ipcRenderer.invoke(IPC_CHANNELS.APP.MAXIMIZE),
    close: () => ipcRenderer.invoke(IPC_CHANNELS.APP.CLOSE),
    toggleFullscreen: () => ipcRenderer.invoke(IPC_CHANNELS.APP.TOGGLE_FULLSCREEN),
    isMaximized: () => ipcRenderer.invoke(IPC_CHANNELS.APP.IS_MAXIMIZED)
  });
}

module.exports = { createAppApi };
