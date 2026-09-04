/**
 * Settings, System Services & Tools Module Facade
 */
import * as quotaTokens from './settings/quota_tokens.js';
import * as cliConsole from './settings/cli_console.js';
import * as workspaceConfig from './settings/workspace_config.js';
import * as tunnel from './settings/tunnel.js';
import * as tools from './settings/tools.js';

Object.assign(window, quotaTokens, cliConsole, workspaceConfig, tunnel, tools);

export * from './settings/quota_tokens.js';
export * from './settings/cli_console.js';
export * from './settings/workspace_config.js';
export * from './settings/tunnel.js';
export * from './settings/tools.js';
