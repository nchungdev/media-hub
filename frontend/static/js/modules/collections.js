/**
 * Collections & Streaming Module Facade
 */
import * as colFilters from './collections/collection_filters.js?v=2.6.1';
import * as colRenderer from './collections/collection_renderer.js?v=2.6.1';
import * as plexGrid from './collections/plex_grid.js?v=2.6.1';
import * as player from './collections/player.js?v=2.6.1';
import * as syncModal from './collections/sync_modal.js?v=2.6.1';

Object.assign(window, colFilters, colRenderer, plexGrid, player, syncModal);

export * from './collections/collection_filters.js?v=2.6.1';
export * from './collections/collection_renderer.js?v=2.6.1';
export * from './collections/plex_grid.js?v=2.6.1';
export * from './collections/player.js?v=2.6.1';
export * from './collections/sync_modal.js?v=2.6.1';



