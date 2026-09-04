/**
 * Subtitles Studio Module Facade
 */
import * as studioProjects from './subtitles/studio_projects.js';
import * as seasonGrid from './subtitles/season_grid.js';
import * as episodeActions from './subtitles/episode_actions.js';
import * as stagingTools from './subtitles/staging_tools.js';

Object.assign(window, studioProjects, seasonGrid, episodeActions, stagingTools);

export * from './subtitles/studio_projects.js';
export * from './subtitles/season_grid.js';
export * from './subtitles/episode_actions.js';
export * from './subtitles/staging_tools.js';
