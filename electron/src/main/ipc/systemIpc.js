/**
 * @file systemIpc.js
 * @description IPC handlers for OS-level integration: Dialogs, Clipboard, Shell, and Notifications.
 */

'use strict';

const { shell, dialog, clipboard, Notification } = require('electron');
const { IPC_CHANNELS } = require('../../shared/constants/ipcChannels');
const { createSuccessResponse, createErrorResponse } = require('../../shared/types/contracts');
const windowManager = require('../windows/windowManager');

/**
 * Registers System & OS integration IPC handlers.
 * @param {Electron.IpcMain} ipcMain
 */
function registerSystemIpcHandlers(ipcMain) {
  ipcMain.handle(IPC_CHANNELS.SYSTEM.OPEN_EXTERNAL, async (_, url) => {
    try {
      if (typeof url === 'string' && (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('magnet:'))) {
        await shell.openExternal(url);
        return createSuccessResponse(true);
      }
      return createErrorResponse('Invalid URL protocol');
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.SHOW_ITEM_IN_FOLDER, (_, fullPath) => {
    try {
      if (typeof fullPath === 'string') {
        shell.showItemInFolder(fullPath);
        return createSuccessResponse(true);
      }
      return createErrorResponse('Invalid file path');
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.SHOW_OPEN_DIALOG, async (_, options = {}) => {
    try {
      const win = windowManager.getMainWindow();
      const result = await dialog.showOpenDialog(win, options);
      return createSuccessResponse(result);
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.SHOW_SAVE_DIALOG, async (_, options = {}) => {
    try {
      const win = windowManager.getMainWindow();
      const result = await dialog.showSaveDialog(win, options);
      return createSuccessResponse(result);
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.READ_CLIPBOARD, () => {
    try {
      const text = clipboard.readText();
      return createSuccessResponse(text);
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.WRITE_CLIPBOARD, (_, text) => {
    try {
      clipboard.writeText(String(text || ''));
      return createSuccessResponse(true);
    } catch (err) {
      return createErrorResponse(err);
    }
  });

  ipcMain.handle(IPC_CHANNELS.SYSTEM.SHOW_NOTIFICATION, (_, { title, body, silent }) => {
    try {
      if (Notification.isSupported()) {
        const notif = new Notification({
          title: title || 'Media Hub',
          body: body || '',
          silent: !!silent
        });
        notif.show();
        return createSuccessResponse(true);
      }
      return createErrorResponse('Desktop notifications not supported');
    } catch (err) {
      return createErrorResponse(err);
    }
  });
}

module.exports = { registerSystemIpcHandlers };
