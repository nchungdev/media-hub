/**
 * Torbox & Downloads Module Facade
 */
import * as torrents from './torbox/torrents.js';
import * as actions from './torbox/actions.js';

Object.assign(window, torrents, actions);

export * from './torbox/torrents.js';
export * from './torbox/actions.js';
