/**
 * API Fetch Client & Network Layer
 */
import { showToast } from './toast.js';

export async function apiFetch(url, options = {}) {
  const defaultHeaders = {};
  if (options.body && typeof options.body === 'object' && !(options.body instanceof FormData)) {
    defaultHeaders['Content-Type'] = 'application/json';
    options.body = JSON.stringify(options.body);
  }

  options.headers = {
    ...defaultHeaders,
    ...options.headers,
  };

  try {
    const res = await fetch(url, options);
    if (!res.ok) {
      let errMsg = `HTTP ${res.status}: ${res.statusText}`;
      try {
        const errJson = await res.json();
        if (errJson.error || errJson.message) {
          errMsg = errJson.error || errJson.message;
        }
      } catch (_) {}
      throw new Error(errMsg);
    }
    const contentType = res.headers.get('content-type') || '';
    if (contentType.includes('application/json')) {
      return await res.json();
    }
    return await res.text();
  } catch (err) {
    console.error(`[API Error] ${url}:`, err);
    throw err;
  }
}

window.apiFetch = apiFetch;
