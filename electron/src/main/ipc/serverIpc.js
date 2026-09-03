/**
 * @file serverIpc.js
 * @description IPC handlers for Python backend status and diagnostics.
 */

'use strict';

const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');
const { createSuccessResponse, createErrorResponse } = require('../../shared/types/contracts');
const appConfig = require('../config/appConfig');
const backendManager = require('../core/backendManager');

/**
 * Registers Backend Server IPC handlers.
 * @param {Electron.IpcMain} ipcMain
 */
function registerServerIpcHandlers(ipcMain) {
  ipcMain.handle(IPC_CHANNELS.SERVER.GET_STATUS, async () => {
    try {
      const isOnline = await backendManager.checkHealth(400);
      return createSuccessResponse({
        online: isOnline,
        port: appConfig.port,
        host: appConfig.host,
        url: appConfig.serverUrl,
        logFile: appConfig.paths.serverLogFile
      });
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SERVER.GET_PORT, () => {
    return createSuccessResponse(appConfig.port);
  });

  ipcMain.handle(IPC_CHANNELS.SERVER.GET_URL, () => {
    return createSuccessResponse(appConfig.serverUrl);
  });

  ipcMain.handle(IPC_CHANNELS.SERVER.GET_LOG_PATH, () => {
    return createSuccessResponse(appConfig.paths.serverLogFile);
  });

  ipcMain.handle(IPC_CHANNELS.SERVER.PING, async () => {
    const isOnline = await backendManager.checkHealth(300);
    return createSuccessResponse(isOnline);
  });

  ipcMain.handle(IPC_CHANNELS.SERVER.RESTART, async () => {
    try {
      const restarted = await backendManager.start();
      return createSuccessResponse(restarted);
    } catch (err) {
      return createErrorResponse(err);
    }
  });
}

module.exports = { registerServerIpcHandlers };
