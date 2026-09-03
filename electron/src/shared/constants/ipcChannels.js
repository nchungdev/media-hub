/**
 * @file ipcChannels.js
 * @description Strongly-typed frozen constants for all IPC channels across Main and Preload/Renderer processes.
 * Adheres to Clean Architecture by maintaining a single source of truth for inter-process communication contracts.
 */

'use strict';

const IPC_CHANNELS = Object.freeze({
  APP: Object.freeze({
    GET_INFO: 'app:get-info',
    GET_PLATFORM: 'app:get-platform',
    MINIMIZE: 'app:minimize-window',
    MAXIMIZE: 'app:maximize-window',
    CLOSE: 'app:close-window',
    TOGGLE_FULLSCREEN: 'app:toggle-fullscreen',
    IS_MAXIMIZED: 'app:is-maximized'
  }),

  SYSTEM: Object.freeze({
    OPEN_EXTERNAL: 'system:open-external',
    SHOW_ITEM_IN_FOLDER: 'system:show-item-in-folder',
    SHOW_OPEN_DIALOG: 'system:show-open-dialog',
    SHOW_SAVE_DIALOG: 'system:show-save-dialog',
    READ_CLIPBOARD: 'system:read-clipboard',
    WRITE_CLIPBOARD: 'system:write-clipboard',
    SHOW_NOTIFICATION: 'system:show-notification'
  }),

  SERVER: Object.freeze({
    GET_STATUS: 'server:get-status',
    GET_PORT: 'server:get-port',
    GET_URL: 'server:get-url',
    GET_LOG_PATH: 'server:get-log-path',
    RESTART: 'server:restart',
    PING: 'server:ping'
  })
});

module.exports = { IPC_CHANNELS };
