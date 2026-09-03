/**
 * @file index.js
 * @description Central Registry for registering all IPC handlers across domains.
 */

'use strict';

const { registerAppIpcHandlers } = require('./appIpc');
const { registerSystemIpcHandlers } = require('./systemIpc');
const { registerServerIpcHandlers } = require('./serverIpc');

/**
 * Registers all IPC handlers onto the Electron ipcMain instance.
 * @param {Electron.IpcMain} ipcMain
 */
function registerAllIpcHandlers(ipcMain) {
  registerAppIpcHandlers(ipcMain);
  registerSystemIpcHandlers(ipcMain);
  registerServerIpcHandlers(ipcMain);
  console.log('[IPC] ✅ All IPC domain handlers registered successfully.');
}

module.exports = { registerAllIpcHandlers };
