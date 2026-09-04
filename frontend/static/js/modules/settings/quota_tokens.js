/**
 * Token Usage Analytics & Quota Guard Controller
 */
import { showToast } from '../../core/toast.js';

    async function loadTokenUsageReport() {
      const tbody = document.getElementById('token-report-tbody');
      loadQuotaGuardStatus();
      try {
        const res = await fetch('/api/agent/token_usage');
        const data = await res.json();
        
        // Update Stat Summary Cards
        if (document.getElementById('token-stat-total')) document.getElementById('token-stat-total').textContent = (data.total_tokens || 0).toLocaleString();
        if (document.getElementById('token-stat-in')) document.getElementById('token-stat-in').textContent = (data.total_input_tokens || 0).toLocaleString();
        if (document.getElementById('token-stat-out')) document.getElementById('token-stat-out').textContent = (data.total_output_tokens || 0).toLocaleString();
        if (document.getElementById('token-stat-cost')) document.getElementById('token-stat-cost').textContent = '$' + (data.total_cost_usd || 0).toFixed(6);
        if (document.getElementById('token-stat-turns')) document.getElementById('token-stat-turns').textContent = ((data.sessions || []).reduce((a, b) => a + (b.turns || 0), 0)).toLocaleString();
        if (document.getElementById('token-stat-tools')) document.getElementById('token-stat-tools').textContent = ((data.sessions || []).reduce((a, b) => a + (b.tool_calls || 0), 0)).toLocaleString();
        if (document.getElementById('token-stat-sessions')) document.getElementById('token-stat-sessions').textContent = (data.total_sessions || 0);

        // Render Sessions Table
        if (!tbody) return;
        if (!data.sessions || data.sessions.length === 0) {
          tbody.innerHTML = `
            <tr>
              <td colspan="9" class="py-8 text-center text-zinc-500 font-sans">
                <span class="text-2xl block mb-2">📭</span>
                Chưa có session AI nào được ghi nhận. Bấm <strong>Dịch Phụ Đề</strong> trên bất kỳ phim nào để bắt đầu ghi nhật ký!
              </td>
            </tr>
          `;
          return;
        }

        tbody.innerHTML = data.sessions.map(s => {
          const isMonster = s.media_id.includes('74599');
          const isThreeEyed = s.media_id.includes('320122');
          const isWataru = s.media_id.includes('446736');
          const friendlyName = isMonster ? 'Monster (2004)' : (isThreeEyed ? 'The Three-Eyed One' : (isWataru ? 'Mashin Creator Wataru' : s.media_id.replace('media-', '')));
          
          return `
            <tr class="hover:bg-zinc-900/40 transition">
              <td class="py-3 pr-2">
                <div class="font-bold text-white font-sans text-xs flex items-center gap-1.5">
                  <span>🎬</span> ${friendlyName}
                </div>
                <div class="text-[10px] text-cyan-400 font-mono">${s.media_id}</div>
              </td>
              <td class="py-3 pr-2 text-zinc-400 font-mono text-[11px]">
                <span class="px-1.5 py-0.5 rounded bg-zinc-950 border border-zinc-800 text-zinc-400">${s.conv_id ? s.conv_id.slice(0, 8) + '...' : '--'}</span>
              </td>
              <td class="py-3 pr-2 text-center text-zinc-300 font-semibold">${s.turns || 0}</td>
              <td class="py-3 pr-2 text-right text-zinc-300 font-mono">${(s.input_tokens || 0).toLocaleString()}</td>
              <td class="py-3 pr-2 text-right text-zinc-300 font-mono">
                ${(s.output_tokens || 0).toLocaleString()}
                ${s.thinking_tokens ? `<span class="text-[10px] text-purple-400 block font-normal">+${s.thinking_tokens.toLocaleString()} think</span>` : ''}
              </td>
              <td class="py-3 pr-2 text-right font-bold text-white font-mono">${(s.total_tokens || 0).toLocaleString()}</td>
              <td class="py-3 pr-2 text-right text-emerald-400 font-bold font-mono">$${(s.est_cost_usd || 0).toFixed(6)}</td>
              <td class="py-3 pr-2 text-center text-[10px] text-zinc-500 font-sans">${s.last_active || '--'}</td>
              <td class="py-3 text-right">
                <button onclick="resetMediaSession('${s.media_id}')" class="px-2.5 py-1 rounded-lg bg-red-600/10 hover:bg-red-600/20 text-red-400 border border-red-500/20 text-[10px] font-sans font-semibold transition" title="Xoá session cache này để tránh phình token">
                  <span>🗑️ Reset</span>
                </button>
              </td>
            </tr>
          `;
        }).join('');
      } catch (e) {
        if (tbody) tbody.innerHTML = `<tr><td colspan="9" class="py-6 text-center text-red-400 font-sans">Lỗi tải dữ liệu: ${e}</td></tr>`;
      }
    }

    async function resetMediaSession(mediaId) {
      if (!confirm(`Bạn có chắc muốn xoá session cache cho [${mediaId}]? Lượt gọi tiếp theo sẽ khởi tạo ngữ cảnh sạch mới.`)) return;
      try {
        const res = await fetch('/api/agent/session/reset', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ media_id: mediaId })
        });
        const data = await res.json();
        if (data.success) {
          showToast('✅ ' + data.message, 'success');
          loadTokenUsageReport();
        } else {
          showToast('❌ Lỗi: ' + (data.error || 'Không thể xóa session'), 'error');
        }
      } catch (e) {
        showToast('❌ Lỗi kết nối: ' + e, 'error');
      }
    }

    // ==================== TRANSLATION QUOTA GUARD ====================
    window.quotaGuardStatus = null;
    window.quotaGuardLocked = false;

    async function loadQuotaGuardStatus() {
      try {
        const res = await fetch('/api/agent/quota_status');
        const data = await res.json();
        window.quotaGuardStatus = data;
        window.quotaGuardLocked = Boolean(data.is_locked);

        // 1. Update Subtitle Studio (Tab 4) Banner
        const subDayUsed = document.getElementById('sub-quota-day-used');
        const subDayLimit = document.getElementById('sub-quota-day-limit');
        const subDayPct = document.getElementById('sub-quota-day-pct');
        const subDayBar = document.getElementById('sub-quota-day-bar');
        const subDayReset = document.getElementById('sub-quota-day-reset');

        const subWeekUsed = document.getElementById('sub-quota-week-used');
        const subWeekLimit = document.getElementById('sub-quota-week-limit');
        const subWeekPct = document.getElementById('sub-quota-week-pct');
        const subWeekBar = document.getElementById('sub-quota-week-bar');
        const subWeekReset = document.getElementById('sub-quota-week-reset');

        const subBadge = document.getElementById('sub-quota-badge');
        const subIcon = document.getElementById('sub-quota-status-icon');

        if (subDayUsed) subDayUsed.textContent = data.day.used;
        if (subDayLimit) subDayLimit.textContent = data.day.limit;
        if (subDayPct) subDayPct.textContent = `${data.day.percentage}%`;
        if (subDayBar) {
          subDayBar.style.width = `${Math.min(100, data.day.percentage)}%`;
          subDayBar.className = `h-full transition-all duration-500 ${data.day.percentage >= 100 ? 'bg-red-500' : (data.day.percentage >= 80 ? 'bg-amber-500' : 'bg-emerald-500')}`;
        }
        if (subDayReset) subDayReset.textContent = `Reset: ${data.day.reset_in}`;

        if (subWeekUsed) subWeekUsed.textContent = data.week.used;
        if (subWeekLimit) subWeekLimit.textContent = data.week.limit;
        if (subWeekPct) subWeekPct.textContent = `${data.week.percentage}%`;
        if (subWeekBar) {
          subWeekBar.style.width = `${Math.min(100, data.week.percentage)}%`;
          subWeekBar.className = `h-full transition-all duration-500 ${data.week.percentage >= 100 ? 'bg-red-500' : (data.week.percentage >= 80 ? 'bg-amber-500' : 'bg-cyan-500')}`;
        }
        if (subWeekReset) subWeekReset.textContent = `Reset: ${data.week.reset_in}`;

        if (subBadge) {
          subBadge.textContent = data.status_label;
          const colorClasses = data.status_code === 'LOCKED' ? 'bg-red-500/10 text-red-400 border-red-500/20' : (data.status_code === 'WARNING' ? 'bg-amber-500/10 text-amber-400 border-amber-500/20' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20');
          subBadge.className = `text-[10px] px-2 py-0.5 rounded-full font-bold font-mono border ${colorClasses}`;
        }
        if (subIcon) {
          subIcon.innerHTML = data.status_code === 'LOCKED' ? '🛑' : (data.status_code === 'WARNING' ? '⚠️' : '🛡️');
          const iconColors = data.status_code === 'LOCKED' ? 'bg-red-500/10 text-red-400 border-red-500/20' : (data.status_code === 'WARNING' ? 'bg-amber-500/10 text-amber-400 border-amber-500/20' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20');
          subIcon.className = `w-10 h-10 rounded-2xl border flex items-center justify-center text-lg font-bold shrink-0 shadow-inner ${iconColors}`;
        }

        // 2. Update Token Usage (Tab 6) Quota Card
        const tokenBadge = document.getElementById('token-quota-badge');
        const tokenDayText = document.getElementById('token-quota-day-text');
        const tokenDayBar = document.getElementById('token-quota-day-bar');
        const tokenDayTokens = document.getElementById('token-quota-day-tokens');
        const tokenDayReset = document.getElementById('token-quota-day-reset');

        const tokenWeekText = document.getElementById('token-quota-week-text');
        const tokenWeekBar = document.getElementById('token-quota-week-bar');
        const tokenWeekTokens = document.getElementById('token-quota-week-tokens');
        const tokenWeekReset = document.getElementById('token-quota-week-reset');

        if (tokenBadge) {
          tokenBadge.textContent = data.status_label;
          const colorClasses = data.status_code === 'LOCKED' ? 'bg-red-500/10 text-red-400 border-red-500/20' : (data.status_code === 'WARNING' ? 'bg-amber-500/10 text-amber-400 border-amber-500/20' : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20');
          tokenBadge.className = `px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${colorClasses}`;
        }
        if (tokenDayText) tokenDayText.textContent = `${data.day.used} / ${data.day.limit} tập (${data.day.percentage}%)`;
        if (tokenDayBar) {
          tokenDayBar.style.width = `${Math.min(100, data.day.percentage)}%`;
          tokenDayBar.className = `h-full transition-all duration-500 ${data.day.percentage >= 100 ? 'bg-red-500' : (data.day.percentage >= 80 ? 'bg-amber-500' : 'bg-emerald-500')}`;
        }
        if (tokenDayTokens) tokenDayTokens.textContent = (data.day.tokens_est || 0).toLocaleString();
        if (tokenDayReset) tokenDayReset.textContent = `Reset sau: ${data.day.reset_in}`;

        if (tokenWeekText) tokenWeekText.textContent = `${data.week.used} / ${data.week.limit} tập (${data.week.percentage}%)`;
        if (tokenWeekBar) {
          tokenWeekBar.style.width = `${Math.min(100, data.week.percentage)}%`;
          tokenWeekBar.className = `h-full transition-all duration-500 ${data.week.percentage >= 100 ? 'bg-red-500' : (data.week.percentage >= 80 ? 'bg-amber-500' : 'bg-cyan-500')}`;
        }
        if (tokenWeekTokens) tokenWeekTokens.textContent = (data.week.tokens_est || 0).toLocaleString();
        if (tokenWeekReset) tokenWeekReset.textContent = `Reset sau: ${data.week.reset_in}`;

        // 3. Populate Settings fields (Tab 5) if empty
        const cfgDay = document.getElementById('cfg-quota-daily');
        const cfgWeek = document.getElementById('cfg-quota-weekly');
        if (cfgDay && (!cfgDay.value || cfgDay.value === '30')) cfgDay.value = data.day.limit;
        if (cfgWeek && (!cfgWeek.value || cfgWeek.value === '150')) cfgWeek.value = data.week.limit;

      } catch (e) {
        console.error('Error loading quota guard status:', e);
      }
    }

    async function resetQuotaGuard(scope) {
      const label = scope === 'day' ? 'ngày hôm nay' : 'toàn bộ (ngày & tuần)';
      if (!confirm(`Bạn có chắc muốn reset bộ đếm Quota cho ${label}?`)) return;
      try {
        const res = await fetch('/api/agent/quota_reset', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ scope: scope })
        });
        const data = await res.json();
        if (data.success) {
          showToast(`✅ ${data.message}`, 'success');
          loadQuotaGuardStatus();
          if (typeof loadSubtitleStudioData === 'function') loadSubtitleStudioData();
        } else {
          showToast('❌ Lỗi reset Quota: ' + (data.error || 'Thất bại'), 'error');
        }
      } catch (e) {
        showToast('❌ Lỗi kết nối: ' + e, 'error');
      }
    }

    // ==================== LIVE CLI CONSOLE TAB ====================
    window.tabConsoleInterval = null;
    window.rawTabConsoleLogs = [];


export {
  loadTokenUsageReport,
  resetMediaSession,
  loadQuotaGuardStatus,
  resetQuotaGuard
};
