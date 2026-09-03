/**
 * @file contracts.js
 * @description Standardized data transfer objects and contract helpers for IPC communication.
 */

'use strict';

/**
 * Creates a standardized success response.
 * @param {*} data
 * @param {string} [message]
 * @returns {{ success: true, data: *, message?: string }}
 */
function createSuccessResponse(data, message = '') {
  const res = { success: true, data };
  if (message) res.message = message;
  return res;
}

/**
 * Creates a standardized error response.
 * @param {string|Error} error
 * @param {string} [code]
 * @returns {{ success: false, error: string, code?: string }}
 */
function createErrorResponse(error, code = '') {
  const message = error instanceof Error ? error.message : String(error);
  const res = { success: false, error: message };
  if (code) res.code = code;
  return res;
}

module.exports = {
  createSuccessResponse,
  createErrorResponse
};
