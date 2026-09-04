/**
 * Toast Notification Engine (Minimal & Unintrusive)
 */

window._lastToastTime = 0;
window._lastToastMsg = '';

export function showToast(message, type = 'info', duration = 2200, force = false) {
  // 1. Suppress routine action in-flight / info notifications unless explicitly forced
  if (type === 'info' && !force) {
    console.log(`[MediaHub Action Info] ${message}`);
    return;
  }

  // 2. Debounce duplicate messages within 2.5 seconds
  const now = Date.now();
  if (message === window._lastToastMsg && (now - window._lastToastTime) < 2500) {
    return;
  }
  window._lastToastMsg = message;
  window._lastToastTime = now;

  let container = document.getElementById('toast-container');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toast-container';
    container.className = 'fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none';
    document.body.appendChild(container);
  }

  // 3. Keep only 1 visible toast at a time (prevent stacking covering the screen)
  while (container.firstChild) {
    container.removeChild(container.firstChild);
  }

  const toastId = 'toast-' + now;

  let borderStyle = "border-zinc-800 bg-zinc-950/95";
  let icon = "ℹ️";
  let barColor = "bg-blue-500";
  let textColor = "text-white";

  if (type === 'success') {
    borderStyle = "border-emerald-500/40 bg-zinc-950/95 shadow-emerald-500/10";
    icon = "✓";
    barColor = "bg-emerald-500";
    textColor = "text-emerald-300";
    if (duration > 2200) duration = 2200; // Concise 2.2s max for success
  } else if (type === 'error') {
    borderStyle = "border-red-500/40 bg-zinc-950/95 shadow-red-500/10";
    icon = "✕";
    barColor = "bg-red-500";
    textColor = "text-red-300";
    duration = Math.max(duration, 3500); // Errors remain visible long enough to read
  } else if (type === 'warning') {
    borderStyle = "border-amber-500/40 bg-zinc-950/95 shadow-amber-500/10";
    icon = "⚡";
    barColor = "bg-amber-500";
    textColor = "text-amber-300";
    duration = Math.max(duration, 3000);
  }

  const toastEl = document.createElement('div');
  toastEl.id = toastId;
  toastEl.className = `pointer-events-auto px-3.5 py-2.5 rounded-2xl border ${borderStyle} shadow-2xl backdrop-blur-xl flex flex-col gap-1.5 transition-all duration-300 transform translate-y-[-8px] opacity-0 text-xs`;

  toastEl.innerHTML = `
    <div class="flex items-center justify-between gap-3">
      <div class="flex items-center gap-2 min-w-0">
        <span class="w-4 h-4 rounded-full flex items-center justify-center font-bold text-[10px] shrink-0 ${barColor} text-black">${icon}</span>
        <div class="${textColor} font-medium leading-snug break-words max-w-xs">${message}</div>
      </div>
      <button onclick="dismissToast('${toastId}')" class="text-zinc-500 hover:text-white p-0.5 rounded-lg hover:bg-zinc-800 transition text-[11px] shrink-0" title="Đóng">
        ✕
      </button>
    </div>
  `;

  container.appendChild(toastEl);

  // Animate in
  requestAnimationFrame(() => {
    toastEl.classList.remove('translate-y-[-8px]', 'opacity-0');
  });

  // Auto dismiss timer
  const timeoutId = setTimeout(() => {
    dismissToast(toastId);
  }, duration);

  toastEl._timeoutId = timeoutId;
}

export function dismissToast(toastId) {
  const toastEl = document.getElementById(toastId);
  if (!toastEl) return;
  if (toastEl._timeoutId) clearTimeout(toastEl._timeoutId);
  toastEl.classList.add('opacity-0', 'translate-y-[-8px]');
  setTimeout(() => {
    if (toastEl && toastEl.parentNode) toastEl.parentNode.removeChild(toastEl);
  }, 250);
}

window.showToast = showToast;
window.dismissToast = dismissToast;
