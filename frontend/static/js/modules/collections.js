/**
 * Collections & Streaming Module Facade
 */
import * as colFilters from './collections/collection_filters.js';
import * as colRenderer from './collections/collection_renderer.js';
import * as plexGrid from './collections/plex_grid.js';
import * as player from './collections/player.js';

Object.assign(window, colFilters, colRenderer, plexGrid, player);

export * from './collections/collection_filters.js';
export * from './collections/collection_renderer.js';
export * from './collections/plex_grid.js';
export * from './collections/player.js';
