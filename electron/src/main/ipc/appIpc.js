/**
 * @file appIpc.js
 * @description IPC handlers for application info and window management.
 */

'use strict';

const { app } = require('electron');
const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');
const { createSuccessResponse } = require('../../shared/types/contracts');
const windowManager = require('../windows/windowManager');

/**
 * Registers Application & Window IPC handlers.
 * @param {Electron.IpcMain} ipcMain
 */
function registerAppIpcHandlers(ipcMain) {
  ipcMain.handle(IPC_CHANNELS.APP.GET_INFO, () => {
    return createSuccessResponse({
      name: app.name,
      version: app.getVersion(),
      electronVersion: process.versions.electron,
      chromeVersion: process.versions.chrome,
      nodeVersion: process.versions.node
    });
  });

  ipcMain.handle(IPC_CHANNELS.APP.GET_PLATFORM, () => {
    return createSuccessResponse({
      platform: process.platform,
      arch: process.arch
    });
  });

  ipcMain.handle(IPC_CHANNELS.APP.MINIMIZE, () => {
    windowManager.minimizeMainWindow();
    return createSuccessResponse(true);
  });

  ipcMain.handle(IPC_CHANNELS.APP.MAXIMIZE, () => {
    windowManager.toggleMaximizeMainWindow();
    return createSuccessResponse(true);
  });

  ipcMain.handle(IPC_CHANNELS.APP.CLOSE, () => {
    windowManager.closeMainWindow();
    return createSuccessResponse(true);
  });

  ipcMain.handle(IPC_CHANNELS.APP.TOGGLE_FULLSCREEN, () => {
    windowManager.toggleFullscreenMainWindow();
    return createSuccessResponse(true);
  });

  ipcMain.handle(IPC_CHANNELS.APP.IS_MAXIMIZED, () => {
    const win = windowManager.getMainWindow();
    return createSuccessResponse(win ? win.isMaximized() : false);
  });
}

module.exports = { registerAppIpcHandlers };
