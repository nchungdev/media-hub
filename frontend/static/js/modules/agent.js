/**
 * AI Agent Bridge Module (Chat, Prompts, Task Queue Watcher)
 */
import { showToast } from '../core/toast.js';

    let isAIChatOpen = false;

    function toggleAIChat() {
      const chat = document.getElementById('floating-ai-chat');
      const btn = document.getElementById('floating-ai-btn');
      if (!chat) return;
      isAIChatOpen = !isAIChatOpen;
      
      if (isAIChatOpen) {
        chat.classList.remove('hidden');
        chat.style.display = 'flex';
        if (btn) {
          btn.classList.add('hidden');
          btn.style.display = 'none';
        }
        loadAIChatMessages();
        setTimeout(() => {
          const inp = document.getElementById('ai-chat-input');
          if (inp) inp.focus();
        }, 150);
      } else {
        chat.classList.add('hidden');
        chat.style.display = 'none';
        if (btn) {
          btn.classList.remove('hidden');
          btn.style.display = 'flex';
        }
      }
    }

    async function sendAIChatMessage() {
      const input = document.getElementById('ai-chat-input');
      const cmd = input.value.trim();
      if (!cmd) return;

      const container = document.getElementById('ai-chat-messages');
      const tempId = Date.now();

      // Render outgoing user message bubble immediately with "⏳ Đang gửi..."
      const userBubble = `
        <div class="flex flex-col items-end gap-1" id="msg-user-${tempId}">
          <div class="p-3 rounded-2xl rounded-tr-none bg-blue-600 text-white max-w-[85%] break-words">
            ${cmd.replace(/</g, '&lt;').replace(/>/g, '&gt;')}
          </div>
          <div class="text-[9px] text-zinc-400 font-mono flex items-center gap-1" id="msg-status-${tempId}">
            <span>⏳ Đang gửi...</span>
          </div>
        </div>
      `;
      container.innerHTML += userBubble;
      input.value = '';
      container.scrollTop = container.scrollHeight;

      try {
        const res = await fetch('/api/agent/command', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({command: cmd})
        });
        const data = await res.json();
        
        // Update user message status to "✓✓ Đã nhận"
        const statusEl = document.getElementById(`msg-status-${tempId}`);
        if (statusEl) {
          statusEl.innerHTML = `<span class="text-blue-400">✓✓ Đã nhận</span> <span>• ${new Date().toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</span>`;
        }

        // Render AI Agent Response Bubble
        setTimeout(() => {
          const aiResponse = (data.command && data.command.response) ? data.command.response : `🤖 Đã tiếp nhận lệnh: "${cmd}". Đang điều phối!`;
          const aiBubble = `
            <div class="flex items-start gap-2">
              <div class="w-6 h-6 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-xs shrink-0">🤖</div>
              <div class="p-3 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-200 max-w-[85%] space-y-1 shadow-md">
                <div>${aiResponse}</div>
                <div class="text-[9px] text-zinc-500 font-mono text-right">${new Date().toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</div>
              </div>
            </div>
          `;
          container.innerHTML += aiBubble;
          container.scrollTop = container.scrollHeight;
        }, 400);

      } catch (e) {
        const statusEl = document.getElementById(`msg-status-${tempId}`);
        if (statusEl) {
          statusEl.innerHTML = `<span class="text-red-400">✕ Gửi thất bại</span>`;
        }
      }
    }

    async function loadAIChatMessages() {
      try {
        const res = await fetch('/api/agent/queue');
        const list = await res.json();
        const container = document.getElementById('ai-chat-messages');
        
        if (!list || list.length === 0) return;

        let html = `
          <div class="flex items-start gap-2">
            <div class="w-6 h-6 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-xs shrink-0">🤖</div>
            <div class="p-3 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-300 space-y-1">
              <div>Xin chào! Tôi là AI Agent điều phối Media Hub. Bạn có thể gửi lệnh điều khiển hoặc hỏi bất cứ điều gì về tiến trình tải phim tại đây.</div>
              <div class="text-[9px] text-zinc-500 font-mono text-right">Trực tuyến</div>
            </div>
          </div>
        `;

        list.forEach(item => {
          let statusText = "✓✓ Đã nhận";
          let statusColor = "text-blue-400";
          if (item.status === 'pending') {
            statusText = "✓ Đã gửi";
            statusColor = "text-zinc-400";
          } else if (item.status === 'done') {
            statusText = "✓✓ Đã hoàn tất";
            statusColor = "text-emerald-400";
          }

          // User message bubble
          html += `
            <div class="flex flex-col items-end gap-1">
              <div class="p-3 rounded-2xl rounded-tr-none bg-blue-600 text-white max-w-[85%] break-words">
                ${item.command}
              </div>
              <div class="text-[9px] text-zinc-400 font-mono flex items-center gap-1">
                <span class="${statusColor}">${statusText}</span>
                <span>• ${item.timestamp || ''}</span>
              </div>
            </div>
          `;

          // AI Agent response bubble if exists
          if (item.response) {
            html += `
              <div class="flex items-start gap-2">
                <div class="w-6 h-6 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-xs shrink-0">🤖</div>
                <div class="p-3 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-200 max-w-[85%] space-y-1 shadow-md">
                  <div>${item.response}</div>
                  <div class="text-[9px] text-zinc-500 font-mono text-right">${item.timestamp || ''}</div>
                </div>
              </div>
            `;
          }
        });

        container.innerHTML = html;
        container.scrollTop = container.scrollHeight;
      } catch (e) {}
    }


    async function sendQuickCommand(cmd, mediaId) {
      const inp = document.getElementById('full-agent-input');
      if (inp) {
        inp.value = cmd;
        sendFullAgentMessage(mediaId);
      }
    }

    async function sendFullAgentMessage(explicitMediaId) {
      const input = document.getElementById('full-agent-input');
      const cmd = input.value.trim();
      if (!cmd) return;

      const container = document.getElementById('full-agent-messages');
      const tempId = Date.now();

      // Render outgoing user message bubble immediately
      const userBubble = `
        <div class="flex flex-col items-end gap-1.5" id="full-msg-user-${tempId}">
          <div class="p-4 rounded-2xl rounded-tr-none bg-blue-600 text-white max-w-2xl shadow-md text-xs leading-relaxed">
            ${cmd.replace(/</g, '&lt;').replace(/>/g, '&gt;')}
          </div>
          <div class="text-[10px] text-zinc-400 font-mono flex items-center gap-1" id="full-msg-status-${tempId}">
            <span>⏳ Đang gửi đến Antigravity AI...</span>
          </div>
        </div>
      `;
      container.innerHTML += userBubble;
      container.scrollTop = container.scrollHeight;
      input.value = '';

      try {
        const payload = {command: cmd};
        if (explicitMediaId) payload.media_id = explicitMediaId;
        const res = await fetch('/api/agent/command', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify(payload)
        });
        const data = await res.json();
        
        const statusEl = document.getElementById(`full-msg-status-${tempId}`);
        if (statusEl) {
          statusEl.innerHTML = `<span class="text-blue-400 font-semibold">✓✓ Đã nhận</span> • <span>${new Date().toLocaleTimeString()}</span>`;
        }

        setTimeout(() => {
          let aiResponse = data.message || "✓ Antigravity AI đã tiếp nhận chỉ thị và đang phân tích tác vụ.";
          const aiBubble = `
            <div class="flex items-start gap-3">
              <div class="w-8 h-8 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-sm shrink-0 font-bold">🤖</div>
              <div class="p-4 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-200 max-w-2xl space-y-1.5 shadow-xl text-xs leading-relaxed">
                <div>${aiResponse}</div>
                <div class="text-[10px] text-zinc-500 font-mono text-right">${new Date().toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</div>
              </div>
            </div>
          `;
          container.innerHTML += aiBubble;
          container.scrollTop = container.scrollHeight;
        }, 400);

      } catch (e) {
        const statusEl = document.getElementById(`full-msg-status-${tempId}`);
        if (statusEl) {
          statusEl.innerHTML = `<span class="text-red-400">✕ Gửi thất bại</span>`;
        }
      }
    }

    async function loadAgentQueueFull() {
      try {
        const res = await fetch('/api/agent/queue');
        const list = await res.json();
        const container = document.getElementById('full-agent-messages');
        if (!list || list.length === 0) return;

        let html = `
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-sm shrink-0 font-bold">🤖</div>
            <div class="p-4 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-300 space-y-2 max-w-2xl">
              <p class="font-bold text-white">Xin chào! Tôi là Antigravity AI Agent.</p>
              <p class="leading-relaxed text-zinc-400">Bạn có thể gửi bất kỳ yêu cầu điều khiển hệ thống, yêu cầu tra cứu torrent, sắp xếp thứ tự tải hoặc xử lý video tại đây. Tôi sẽ phản hồi và xử lý tức thì.</p>
              <div class="text-[10px] text-zinc-500 font-mono">Hệ thống sẵn sàng tiếp nhận lệnh</div>
            </div>
          </div>
        `;

        list.forEach(item => {
          let statusText = "✓✓ Đã nhận";
          let statusColor = "text-blue-400";
          if (item.status === 'pending') {
            statusText = "✓ Đã gửi";
            statusColor = "text-zinc-400";
          } else if (item.status === 'done') {
            statusText = "✓✓ Đã hoàn tất";
            statusColor = "text-emerald-400";
          }

          html += `
            <div class="flex flex-col items-end gap-1.5">
              <div class="p-4 rounded-2xl rounded-tr-none bg-blue-600 text-white max-w-2xl text-xs leading-relaxed shadow-md">
                ${item.command}
              </div>
              <div class="text-[10px] text-zinc-400 font-mono flex items-center gap-1">
                <span class="${statusColor}">${statusText}</span>
                <span>• ${item.timestamp || ''}</span>
              </div>
            </div>
          `;

          if (item.response) {
            html += `
              <div class="flex items-start gap-3">
                <div class="w-8 h-8 rounded-full bg-blue-500/20 text-blue-400 flex items-center justify-center text-sm shrink-0 font-bold">🤖</div>
                <div class="p-4 rounded-2xl rounded-tl-none bg-zinc-900 border border-zinc-800 text-zinc-200 max-w-2xl space-y-1.5 shadow-xl text-xs leading-relaxed">
                  <div>${item.response}</div>
                  <div class="text-[10px] text-zinc-500 font-mono text-right">${item.timestamp || ''}</div>
                </div>
              </div>
            `;
          }
        });

        container.innerHTML = html;
        container.scrollTop = container.scrollHeight;
      } catch (e) {}
    }


// Expose functions to window
window.toggleAIChat = toggleAIChat;
window.sendAIChatMessage = sendAIChatMessage;
window.loadAIChatMessages = loadAIChatMessages;
window.sendQuickCommand = sendQuickCommand;
window.sendFullAgentMessage = sendFullAgentMessage;
window.loadAgentQueueFull = loadAgentQueueFull;
