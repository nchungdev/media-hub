/**
 * @file pythonFinder.js
 * @description Strategy pattern for detecting and caching the optimal Python 3 executable binary on the host system.
 */

'use strict';

const fs = require('fs');

class PythonFinder {
  constructor() {
    this._cachedBinary = null;
    this._candidates = [
      '/opt/homebrew/bin/python3',                         // Homebrew Apple Silicon
      '/usr/local/bin/python3',                           // Homebrew Intel
      '/usr/bin/python3',                                 // macOS System Python
      '/Library/Developer/CommandLineTools/usr/bin/python3', // Xcode CLT
      'python3'                                           // PATH Fallback
    ];
  }

  /**
   * Discovers the best Python binary available.
   * @returns {string} Absolute path or 'python3' command
   */
  findBinary() {
    if (this._cachedBinary) {
      return this._cachedBinary;
    }

    for (const candidate of this._candidates) {
      try {
        if (candidate.startsWith('/') && fs.existsSync(candidate)) {
          this._cachedBinary = candidate;
          return candidate;
        }
      } catch (_) {}
    }

    this._cachedBinary = 'python3';
    return this._cachedBinary;
  }

  /**
   * Clears the cached binary path (useful for testing or env changes).
   */
  clearCache() {
    this._cachedBinary = null;
  }
}

// Export singleton instance
module.exports = new PythonFinder();
