/**
 * Overview & Dashboard Module Facade
 */
import * as pipelines from './overview/pipelines.js';
import * as dashboard from './overview/dashboard.js';
import * as dataSync from './overview/data_sync.js';
import * as crossStorage from './overview/cross_storage.js';

Object.assign(window, pipelines, dashboard, dataSync, crossStorage);

export * from './overview/pipelines.js';
export * from './overview/dashboard.js';
export * from './overview/data_sync.js';
export * from './overview/cross_storage.js';
