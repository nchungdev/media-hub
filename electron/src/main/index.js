/**
 * @file index.js
 * @description Application Composition Root.
 * Orchestrates the application lifecycle, wires configuration, core infrastructure, IPC, and presentation.
 */

'use strict';

const { app, ipcMain, dialog } = require('electron');
const appConfig = require('./config/appConfig');
const { createApplicationMenu } = require('./config/menu');
const backendManager = require('./core/backendManager');
const windowManager = require('./windows/windowManager');
const { registerAllIpcHandlers } = require('./ipc');

// Ensure single instance lock
const hasLock = app.requestSingleInstanceLock();
if (!hasLock) {
  console.log('[Main] Another instance is already running. Quitting.');
  app.quit();
} else {
  app.on('second-instance', () => {
    // Focus existing window if a second instance attempts to launch
    const win = windowManager.getMainWindow();
    if (win) {
      if (win.isMinimized()) win.restore();
      win.focus();
    }
  });

  // App initialization when Electron runtime is ready
  app.whenReady().then(async () => {
    console.log(`[Main] 🚀 Initializing ${appConfig.appName} (Electron v${process.versions.electron})...`);

    // 1. Setup Native Application Menus
    createApplicationMenu();

    // 2. Register all IPC Domain Handlers
    registerAllIpcHandlers(ipcMain);

    // 3. Start Sidecar Python Backend Server
    const isServerReady = await backendManager.start();
    if (!isServerReady) {
      dialog.showErrorBox(
        'Lỗi Khởi Chạy Máy Chủ',
        `Không thể kết nối với máy chủ Media Hub tại ${appConfig.serverUrl}.\nVui lòng kiểm tra môi trường Python 3 và thử lại.`
      );
    }

    // 4. Create Main Application Window
    windowManager.ensureMainWindow();

    // 5. Re-create window when dock icon is clicked on macOS
    app.on('activate', () => {
      windowManager.ensureMainWindow();
    });
  });

  // Handle cross-platform window closing behavior
  app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
      app.quit();
    }
  });

  // Gracefully handle app exit while leaving background daemon running
  app.on('will-quit', () => {
    backendManager.handleAppQuit();
  });
}
