/**
 * Live CLI Console Terminal & Process Controller
 */
import { showToast } from '../../core/toast.js';

window.tabConsoleInterval = null;
let currentTabConsoleFilter = "all";
let rawTabConsoleLines = [];

    async function pollTabConsoleLogs(forceScroll = false) {
      try {
        // 1. Fetch live events from /api/agy/events and status from /api/agy/status
        const [eventsRes, statusRes, legacyRes] = await Promise.all([
          fetch('/api/agy/events').catch(() => null),
          fetch('/api/agy/status').catch(() => null),
          fetch('/api/agent/live_logs').catch(() => null)
        ]);

        let agyEvents = [];
        if (eventsRes && eventsRes.ok) {
          const eventsData = await eventsRes.json().catch(() => null);
          if (eventsData && Array.isArray(eventsData.events)) {
            agyEvents = eventsData.events;
          }
        }

        let agyStatus = null;
        if (statusRes && statusRes.ok) {
          agyStatus = await statusRes.json().catch(() => null);
        }

        let legacyData = null;
        if (legacyRes && legacyRes.ok) {
          legacyData = await legacyRes.json().catch(() => null);
        }

        if (agyEvents.length > 0) {
          window.rawTabConsoleLogs = agyEvents.map((ev, idx) => {
            // CRITICAL: Dùng khóa event chứ không phải type theo chuẩn Antigravity stream-json
            const eventKey = ev.event !== undefined ? ev.event : (ev.type || 'info');

            const t = ev.time || ev.timestamp || (ev.created_at ? new Date(ev.created_at).toLocaleTimeString() : new Date().toLocaleTimeString());

            let txt = '';
            if (typeof ev.content === 'string') {
              txt = ev.content;
            } else if (Array.isArray(ev.content)) {
              txt = ev.content.map(c => typeof c === 'string' ? c : (c.text || JSON.stringify(c))).join('\n');
            } else if (ev.text) {
              txt = ev.text;
            } else if (ev.message) {
              txt = ev.message;
            } else if (ev.name || ev.tool || ev.tool_call) {
              const tName = ev.name || ev.tool || ev.tool_call?.name || 'tool';
              const args = ev.args || ev.parameters || ev.tool_call?.args;
              txt = `⚙️ Gọi Tool [${tName}]` + (args ? `: ${typeof args === 'object' ? JSON.stringify(args) : args}` : '');
            } else if (ev.result !== undefined || ev.output !== undefined) {
              const resVal = ev.result !== undefined ? ev.result : ev.output;
              txt = typeof resVal === 'object' ? JSON.stringify(resVal) : String(resVal);
            } else if (ev.error) {
              txt = typeof ev.error === 'object' ? (ev.error.message || JSON.stringify(ev.error)) : String(ev.error);
            } else {
              const clean = { ...ev };
              delete clean.event;
              delete clean.type;
              delete clean.time;
              delete clean.timestamp;
              txt = Object.keys(clean).length > 0 ? JSON.stringify(clean) : String(eventKey);
            }

            let lvl = 'info';
            const ekLower = String(eventKey).toLowerCase();
            if (ekLower === 'thought' || ekLower === 'thinking') {
              lvl = 'thinking';
            } else if (ekLower.includes('tool')) {
              lvl = 'tool';
            } else if (ekLower === 'subagent' || ekLower.includes('agent')) {
              lvl = 'subagent';
            } else if (ekLower === 'user') {
              lvl = 'system';
            } else if (ekLower === 'assistant' || ekLower === 'output' || ekLower === 'message') {
              lvl = 'output';
            } else if (ekLower === 'error' || ekLower.includes('fail')) {
              lvl = 'error';
            } else if (ekLower === 'warning' || ekLower === 'warn') {
              lvl = 'warning';
            } else if (ekLower === 'status' || ekLower === 'system') {
              lvl = 'system';
            } else if (ekLower === 'success') {
              lvl = 'success';
            }

            return {
              time: t,
              text: txt,
              level: lvl,
              event: eventKey
            };
          });
        } else if (legacyData && Array.isArray(legacyData.logs) && legacyData.logs.length > 0) {
          window.rawTabConsoleLogs = legacyData.logs;
        } else if (!window.rawTabConsoleLogs) {
          window.rawTabConsoleLogs = [];
        }
        
        // Update indicators
        const badge = document.getElementById('tab-console-status-badge');
        const sideDot = document.getElementById('tab-dot-console');
        const activeBar = document.getElementById('tab-console-active-bar');

        const isRunning = (agyStatus && agyStatus.running) || (legacyData && legacyData.is_running);
        const activeProfile = (agyStatus && agyStatus.profile) || (legacyData?.active_job?.cli) || window.currentAgyProfile || 'agy';

        if (isRunning) {
          if (badge) {
            badge.className = 'px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-400 text-[10px] font-mono font-bold flex items-center gap-1.5 border border-emerald-500/30 animate-pulse';
            badge.innerHTML = `<span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping"></span> RUNNING (${activeProfile})`;
          }
          if (sideDot) {
            sideDot.className = 'w-2 h-2 rounded-full bg-emerald-400 animate-ping shrink-0';
          }
          if (activeBar) {
            activeBar.classList.remove('hidden');
            if (document.getElementById('tab-active-cmd')) {
              document.getElementById('tab-active-cmd').textContent = legacyData?.active_job?.command || `agy stream-json (${activeProfile})`;
            }
            if (document.getElementById('tab-active-cli')) {
              document.getElementById('tab-active-cli').textContent = activeProfile;
            }
            if (document.getElementById('tab-active-time')) {
              document.getElementById('tab-active-time').textContent = 'Sự kiện: ' + (agyStatus?.events_buffered ?? window.rawTabConsoleLogs.length);
            }
            if (document.getElementById('tab-active-context')) {
              const ctx = legacyData?.active_job?.media_id || 'daemon';
              document.getElementById('tab-active-context').textContent = '🎯 ' + ctx;
            }
          }
          updateCliToggleBtn(true);
        } else {
          if (badge) {
            badge.className = 'px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-400 text-[10px] font-mono font-bold flex items-center gap-1.5 border border-zinc-700';
            badge.innerHTML = `<span class="w-1.5 h-1.5 rounded-full bg-zinc-500"></span> IDLE (${activeProfile})`;
          }
          if (sideDot) {
            sideDot.className = 'w-2 h-2 rounded-full bg-zinc-600 shrink-0';
          }
          if (activeBar) {
            activeBar.classList.add('hidden');
          }
          updateCliToggleBtn(false);
        }

        renderTabConsoleLogs(forceScroll);
      } catch (e) {
        console.error('Error polling tab console logs:', e);
      }
    }

    function updateCliToggleBtn(isRunning) {
      const btn = document.getElementById('btn-cli-toggle');
      const icon = document.getElementById('btn-cli-toggle-icon');
      const text = document.getElementById('btn-cli-toggle-text');
      if (!btn) return;
      btn.classList.remove('hidden');
      if (isRunning) {
        btn.className = 'px-3.5 py-1.5 text-xs font-semibold bg-red-600/20 hover:bg-red-600/30 border border-red-500/30 text-red-400 rounded-xl transition flex items-center gap-1.5 shadow-sm font-mono';
        if (icon) icon.textContent = '⏹';
        if (text) text.textContent = 'Dừng CLI';
      } else {
        btn.className = 'px-3.5 py-1.5 text-xs font-semibold bg-emerald-600/20 hover:bg-emerald-600/30 border border-emerald-500/30 text-emerald-400 rounded-xl transition flex items-center gap-1.5 shadow-sm font-mono';
        if (icon) icon.textContent = '▶️';
        if (text) text.textContent = 'Chạy CLI';
      }
      btn.dataset.running = isRunning ? '1' : '0';
    }

    async function toggleCliProcess() {
      const btn = document.getElementById('btn-cli-toggle');
      const isRunning = btn && btn.dataset.running === '1';
      try {
        if (isRunning) {
          btn.disabled = true;
          btn.style.opacity = '0.5';
          await fetch('/api/agent/stop', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' });
        } else {
          btn.disabled = true;
          btn.style.opacity = '0.5';
          await fetch('/api/agent/resume', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' });
        }
        setTimeout(() => { pollTabConsoleLogs(true); btn.disabled = false; btn.style.opacity = '1'; }, 500);
      } catch (e) {
        console.error('toggleCliProcess error:', e);
        if (btn) { btn.disabled = false; btn.style.opacity = '1'; }
      }
    }

    function renderTabConsoleLogs(forceScroll = false) {
      const output = document.getElementById('tab-console-output');
      const countEl = document.getElementById('tab-console-line-count');
      const filterInput = document.getElementById('tab-console-filter');
      const filterText = filterInput ? filterInput.value.trim().toLowerCase() : '';

      if (!output) return;

      let logs = window.rawTabConsoleLogs || [];
      if (countEl) countEl.textContent = logs.length;

      if (filterText) {
        logs = logs.filter(l => (l.text || '').toLowerCase().includes(filterText) || (l.level || '').toLowerCase().includes(filterText));
      }

      if (logs.length === 0) {
        if (filterText) {
          output.innerHTML = `<div class="text-zinc-500 select-none py-4">/* Không tìm thấy log khớp với từ khóa "${filterText}" */</div>`;
        } else {
          output.innerHTML = '<div class="text-zinc-600 select-none">/* --- Antigravity CLI Live Console Initialized. Chưa có log mới. --- */</div>';
        }
        return;
      }

      output.innerHTML = logs.map(l => {
        let colorClass = "text-zinc-300";
        let bgClass = "";
        
        if (l.level === "thinking") {
          colorClass = "text-amber-300/90 italic font-mono text-[11px] leading-relaxed";
          bgClass = "bg-amber-500/[0.04] border-l-2 border-amber-500/40 pl-2 my-0.5";
        } else if (l.level === "tool") {
          colorClass = "text-sky-300 font-mono font-medium text-[11px]";
          bgClass = "bg-sky-500/[0.03] pl-1.5";
        } else if (l.level === "subagent") {
          colorClass = "text-emerald-300 font-mono font-semibold text-xs";
          bgClass = "bg-emerald-500/[0.06] border-l-2 border-emerald-500/40 pl-2 my-0.5";
        } else if (l.level === "output") {
          colorClass = "text-zinc-400 font-mono text-[11px]";
          bgClass = "pl-3";
        } else if (l.level === "system") {
          colorClass = "text-cyan-400 font-semibold font-mono text-xs";
        } else if (l.level === "success") {
          colorClass = "text-emerald-400 font-semibold font-mono text-xs";
        } else if (l.level === "warning") {
          colorClass = "text-amber-400 font-mono text-xs";
        } else if (l.level === "error") {
          colorClass = "text-red-400 font-bold font-mono text-xs";
        }
        
        let eventBadge = '';
        if (l.event && l.event !== 'raw' && l.event !== 'info') {
          eventBadge = `<span class="px-1.5 py-0.2 rounded text-[9px] font-mono font-bold uppercase opacity-85 ${colorClass} bg-zinc-900/90 border border-zinc-800 shrink-0 select-none">${l.event}</span>`;
        }
        
        return `<div class="flex items-start gap-2 hover:bg-zinc-900/50 py-0.5 px-1.5 rounded transition ${bgClass}">
          <span class="text-zinc-600 select-none shrink-0 font-mono text-[10px] pt-0.5">[${l.time}]</span>
          ${eventBadge}
          <span class="${colorClass} break-all select-text font-mono text-xs flex-1">${(l.text || '').replace(/</g, '&lt;').replace(/>/g, '&gt;')}</span>
        </div>`;
      }).join('');

      const chk = document.getElementById('tab-autoscroll-chk');
      if (forceScroll || (chk && chk.checked)) {
        output.scrollTop = output.scrollHeight;
      }
    }

    function filterTabConsoleLogs() {
      renderTabConsoleLogs(false);
    }

    async function clearTabConsole() {
      try {
        await fetch('/api/agent/live_logs/clear', {method: 'POST'}).catch(() => null);
        window.rawTabConsoleLogs = [];
        renderTabConsoleLogs(true);
        showToast('🧹 Đã xóa sạch console log', 'info');
      } catch (e) {}
    }

    function copyTabConsoleLogs() {
      const output = document.getElementById('tab-console-output');
      if (!output) return;
      const text = output.innerText;
      navigator.clipboard.writeText(text).then(() => {
        showToast('📋 Đã sao chép toàn bộ logs vào Clipboard!', 'success');
      });
    }

    function openLiveConsoleModal() {
      setTab('console');
    }


export {
  pollTabConsoleLogs,
  updateCliToggleBtn,
  toggleCliProcess,
  renderTabConsoleLogs,
  filterTabConsoleLogs,
  clearTabConsole,
  copyTabConsoleLogs,
  openLiveConsoleModal
};
