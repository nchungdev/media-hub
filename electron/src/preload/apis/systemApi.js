/**
 * @file systemApi.js
 * @description Preload API wrapper for native OS operations: Dialogs, Clipboard, Shell, Notifications.
 */

'use strict';

const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');

/**
 * Creates the System API bridge.
 * @param {Electron.IpcRenderer} ipcRenderer
 */
function createSystemApi(ipcRenderer) {
  return Object.freeze({
    openExternal: (url) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.OPEN_EXTERNAL, url),
    showItemInFolder: (path) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.SHOW_ITEM_IN_FOLDER, path),
    showOpenDialog: (options) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.SHOW_OPEN_DIALOG, options),
    showSaveDialog: (options) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.SHOW_SAVE_DIALOG, options),
    readClipboard: () => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.READ_CLIPBOARD),
    writeClipboard: (text) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.WRITE_CLIPBOARD, text),
    showNotification: (options) => ipcRenderer.invoke(IPC_CHANNELS.SYSTEM.SHOW_NOTIFICATION, options)
  });
}

module.exports = { createSystemApi };
