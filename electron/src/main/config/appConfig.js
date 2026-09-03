/**
 * @file appConfig.js
 * @description Centralized configuration settings and path resolver for Antigravity Media Hub.
 */

'use strict';

const path = require('path');
const fs = require('fs');
const { app } = require('electron');

class AppConfig {
  constructor() {
    this.port = parseInt(process.env.PORT || '8888', 10);
    this.host = '127.0.0.1';
    this.serverUrl = `http://${this.host}:${this.port}`;
    this.appName = 'Media Hub';
    this.appId = 'com.antigravity.mediahub';

    // Window defaults
    this.window = Object.freeze({
      width: 1440,
      height: 920,
      minWidth: 1024,
      minHeight: 700,
      backgroundColor: '#09090b',
      titleBarStyle: 'hiddenInset',
      trafficLightPosition: { x: 18, y: 18 }
    });

    // Resolve paths dynamically
    this._resolvePaths();
  }

  _resolvePaths() {
    const isPackaged = app.isPackaged;
    const homeDir = process.env.HOME || '';

    // Primary project root (when running unpackaged) vs Resources directory
    let rootDir = path.resolve(__dirname, '..', '..', '..'); // apps/media-hub
    let scriptPath = path.join(rootDir, 'scripts', 'server.py');

    if (isPackaged || !fs.existsSync(scriptPath)) {
      const resourceDir = process.resourcesPath || path.resolve(__dirname, '..', '..', '..');
      const candScript = path.join(resourceDir, 'scripts', 'server.py');
      if (fs.existsSync(candScript)) {
        rootDir = resourceDir;
        scriptPath = candScript;
      }
    }

    this.paths = Object.freeze({
      rootDir,
      scriptPath,
      stateDir: path.join(homeDir, '.media-hub'),
      logDir: path.join(homeDir, '.media-hub', '.logs'),
      serverLogFile: path.join(homeDir, '.media-hub', '.logs', 'server.log'),
      preloadPath: path.resolve(__dirname, '..', '..', 'preload', 'index.js')
    });

    // Ensure log directory exists
    try {
      fs.mkdirSync(this.paths.logDir, { recursive: true });
    } catch (_) {}
  }
}

// Export singleton instance
module.exports = new AppConfig();
