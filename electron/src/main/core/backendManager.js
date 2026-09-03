/**
 * @file backendManager.js
 * @description Singleton Service managing the lifecycle of the Python sidecar daemon.
 * Handles process spawning, detached execution, health checking, and automatic recovery.
 */

'use strict';

const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');
const appConfig = require('../config/appConfig');
const pythonFinder = require('./pythonFinder');

class BackendManager {
  constructor() {
    this._process = null;
    this._isStarting = false;
  }

  /**
   * Pings the Python server to check if it is active and responding.
   * @param {number} [timeoutMs=500]
   * @returns {Promise<boolean>}
   */
  checkHealth(timeoutMs = 500) {
    return new Promise((resolve) => {
      const req = http.get(`${appConfig.serverUrl}/api/settings`, (res) => {
        resolve(res.statusCode === 200);
      });

      req.on('error', () => resolve(false));
      req.setTimeout(timeoutMs, () => {
        req.destroy();
        resolve(false);
      });
    });
  }

  /**
   * Polls the server until it responds or max retries are exceeded.
   * @param {number} [retries=30]
   * @param {number} [intervalMs=350]
   * @returns {Promise<boolean>}
   */
  async waitForReady(retries = 30, intervalMs = 350) {
    for (let attempt = 1; attempt <= retries; attempt++) {
      const isUp = await this.checkHealth(intervalMs);
      if (isUp) return true;
      await new Promise((r) => setTimeout(r, intervalMs));
    }
    return false;
  }

  /**
   * Starts the Python backend daemon process if not already running.
   * @returns {Promise<boolean>} Resolves true if server is online and ready
   */
  async start() {
    // 1. Check if backend is already alive on the target port
    const alreadyUp = await this.checkHealth(250);
    if (alreadyUp) {
      console.log(`[BackendManager] ⚡ Server is already running at ${appConfig.serverUrl}`);
      this._attachCliService();
      return true;
    }

    if (this._isStarting) {
      return this.waitForReady();
    }

    this._isStarting = true;

    try {
      const pythonBin = pythonFinder.findBinary();
      const { scriptPath, rootDir, serverLogFile } = appConfig.paths;

      console.log('[BackendManager] 🚀 Spawning Python Backend Server:');
      console.log('   - Python Binary:', pythonBin);
      console.log('   - Script Path:  ', scriptPath);
      console.log('   - App Directory:', rootDir);
      console.log('   - Log File:     ', serverLogFile);

      const extraPath = [
        '/opt/homebrew/bin',
        '/opt/homebrew/sbin',
        '/usr/local/bin',
        '/usr/bin',
        '/bin',
        '/usr/sbin',
        '/sbin',
        path.join(process.env.HOME || '', '.local', 'bin')
      ].join(':');

      const env = Object.assign({}, process.env, {
        PORT: String(appConfig.port),
        PYTHONUNBUFFERED: '1',
        PATH: extraPath + (process.env.PATH ? ':' + process.env.PATH : '')
      });

      const outLogFd = fs.openSync(serverLogFile, 'a');

      this._process = spawn(pythonBin, [scriptPath], {
        cwd: rootDir,
        env: env,
        stdio: ['ignore', outLogFd, outLogFd],
        detached: true
      });

      this._process.unref();

      console.log(`[BackendManager] ✅ Spawned detached daemon (PID: ${this._process.pid})`);

      const ready = await this.waitForReady(35, 400);
      if (ready) {
        this._attachCliService();
      }
      return ready;
    } catch (err) {
      console.error('[BackendManager] ❌ Failed to start Python backend:', err);
      return false;
    } finally {
      this._isStarting = false;
    }
  }

  /**
   * Sends background request to attach to CLI agent service.
   * @private
   */
  _attachCliService() {
    try {
      const req = http.request(`${appConfig.serverUrl}/api/agent/service/ensure`, { method: 'POST' }, (res) => {
        console.log(`[BackendManager] CLI Service ensure/attach response status: ${res.statusCode}`);
      });
      req.on('error', () => {});
      req.end();
    } catch (_) {}
  }

  /**
   * Detaches process gracefully on application exit so background tasks continue.
   */
  handleAppQuit() {
    if (this._process) {
      console.log('[BackendManager] 💤 Server process detached — keeping alive in background.');
      this._process.unref();
    }
  }
}

// Export singleton instance
module.exports = new BackendManager();
