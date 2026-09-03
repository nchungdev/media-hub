/**
 * @file serverApi.js
 * @description Preload API wrapper for Python server status and diagnostics.
 */

'use strict';

const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');

/**
 * Creates the Server API bridge.
 * @param {Electron.IpcRenderer} ipcRenderer
 */
function createServerApi(ipcRenderer) {
  return Object.freeze({
    getStatus: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.GET_STATUS),
    getPort: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.GET_PORT),
    getUrl: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.GET_URL),
    getLogPath: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.GET_LOG_PATH),
    ping: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.PING),
    restart: () => ipcRenderer.invoke(IPC_CHANNELS.SERVER.RESTART)
  });
}

module.exports = { createServerApi };
