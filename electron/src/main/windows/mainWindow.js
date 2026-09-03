/**
 * @file mainWindow.js
 * @description Factory function for creating and configuring the primary Application BrowserWindow.
 */

'use strict';

const { BrowserWindow, shell } = require('electron');
const appConfig = require('../config/appConfig');

/**
 * Creates and configures the Main Application Window.
 * @returns {BrowserWindow}
 */
function createMainWindow() {
  const win = new BrowserWindow({
    width: appConfig.window.width,
    height: appConfig.window.height,
    minWidth: appConfig.window.minWidth,
    minHeight: appConfig.window.minHeight,
    backgroundColor: appConfig.window.backgroundColor,
    titleBarStyle: appConfig.window.titleBarStyle,
    trafficLightPosition: appConfig.window.trafficLightPosition,
    show: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: false,
      preload: appConfig.paths.preloadPath
    }
  });

  // Load the Python Web Server URL
  win.loadURL(appConfig.serverUrl);

  // Smooth show when DOM is painted to avoid white flash
  win.once('ready-to-show', () => {
    win.show();
  });

  // Intercept external URLs (e.g., Plex, Jellyfin, GitHub, TVDB) and open in default browser
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://')) {
      if (!url.includes(`127.0.0.1:${appConfig.port}`)) {
        shell.openExternal(url);
        return { action: 'deny' };
      }
    }
    return { action: 'allow' };
  });

  return win;
}

module.exports = { createMainWindow };
