/**
 * @file windowManager.js
 * @description Singleton managing active BrowserWindow instances throughout the application lifecycle.
 */

'use strict';

const { createMainWindow } = require('./mainWindow');

class WindowManager {
  constructor() {
    this._mainWindow = null;
  }

  /**
   * Returns the current main window instance or null if destroyed.
   * @returns {Electron.BrowserWindow|null}
   */
  getMainWindow() {
    return this._mainWindow && !this._mainWindow.isDestroyed() ? this._mainWindow : null;
  }

  /**
   * Creates the primary main window or focuses existing window.
   * @returns {Electron.BrowserWindow}
   */
  ensureMainWindow() {
    if (this._mainWindow && !this._mainWindow.isDestroyed()) {
      if (this._mainWindow.isMinimized()) {
        this._mainWindow.restore();
      }
      this._mainWindow.focus();
      return this._mainWindow;
    }

    this._mainWindow = createMainWindow();

    this._mainWindow.on('closed', () => {
      this._mainWindow = null;
    });

    return this._mainWindow;
  }

  /**
   * Reloads the main window content.
   */
  reloadMainWindow() {
    const win = this.getMainWindow();
    if (win) {
      win.reload();
    }
  }

  /**
   * Minimizes the active main window.
   */
  minimizeMainWindow() {
    const win = this.getMainWindow();
    if (win && win.isMinimizable()) {
      win.minimize();
    }
  }

  /**
   * Maximizes or unmaximizes the active main window.
   */
  toggleMaximizeMainWindow() {
    const win = this.getMainWindow();
    if (win && win.isMaximizable()) {
      if (win.isMaximized()) {
        win.unmaximize();
      } else {
        win.maximize();
      }
    }
  }

  /**
   * Closes the active main window.
   */
  closeMainWindow() {
    const win = this.getMainWindow();
    if (win) {
      win.close();
    }
  }

  /**
   * Toggles fullscreen state on the active main window.
   */
  toggleFullscreenMainWindow() {
    const win = this.getMainWindow();
    if (win) {
      win.setFullScreen(!win.isFullScreen());
    }
  }
}

// Export singleton instance
module.exports = new WindowManager();
