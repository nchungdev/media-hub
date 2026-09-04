/**
 * Torbox Download & Sync Actions
 */
import { showToast } from '../../core/toast.js';

    async function queueSyncJob(ids, targets, names, routeLabel) {
      try {
        const res = await fetch('/api/download/sync', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ ids: ids, targets: targets, names: names })
        });
        const data = await res.json();
        if (data.success) {
          showToast(data.message || `\u2705 \u0110\u00e3 x\u1ebfp h\u00e0ng ${routeLabel}!`, 'success', 4000);
          setTimeout(fetchTorrents, 800);
        } else {
          showToast(`\u274c ${data.error || 'L\u1ed7i x\u1ebfp h\u00e0ng \u0111\u1ed3ng b\u1ed9'}`, 'error');
        }
        return data;
      } catch (e) {
        showToast(`\u274c L\u1ed7i k\u1ebft n\u1ed1i: ${e}`, 'error');
        return null;
      }
    }

    async function syncSingleTorrent(id, target, name) {
      const label = target === 'nas' ? 'NAS Storage' : 'Google Drive';
      showToast(`\ud83d\ude80 \u0110ang x\u1ebfp h\u00e0ng \u0111\u1ed3ng b\u1ed9 l\u00ean ${label}...`, 'info', 2500);
      await queueSyncJob([id], [target], [name], label);
    }

    async function syncAllTorrents(target) {
      const label = target === 'nas' ? 'NAS Storage' : 'Google Drive';
      const items = (currentTorrents || []);
      if (items.length === 0) { showToast('Kh\u00f4ng c\u00f3 torrent n\u00e0o \u0111\u1ec3 \u0111\u1ed3ng b\u1ed9.', 'info'); return; }
      if (!confirm(`\u0110\u1ed3ng b\u1ed9 to\u00e0n b\u1ed9 ${items.length} torrents l\u00ean ${label}?`)) return;
      await queueSyncJob(items.map(t => t.id), [target], items.map(t => t.name || ''), label);
    }

    async function syncSelectedTorbox(target) {
      const boxes = Array.from(document.querySelectorAll('.torbox-item-cb:checked'));
      if (boxes.length === 0) { showToast('Ch\u01b0a ch\u1ecdn m\u1ee5c n\u00e0o.', 'info'); return; }
      const label = target === 'nas' ? 'NAS Storage' : 'Google Drive';
      const ids = boxes.map(cb => parseInt(cb.value, 10));
      const names = ids.map(id => (currentTorrents.find(t => t.id === id) || {}).name || '');
      await queueSyncJob(ids, [target], names, label);
    }

    // ---- batch-selection helpers (referenced by onclick but previously never defined,
    //      so every checkbox click threw a ReferenceError) ----
    function updateTorboxSelection() {
      const total = document.querySelectorAll('.torbox-item-cb').length;
      const picked = document.querySelectorAll('.torbox-item-cb:checked').length;
      const counter = document.getElementById('torbox-selected-count');
      if (counter) counter.textContent = `${picked} chọn`;
      const toolbar = document.getElementById('torbox-batch-toolbar');
      if (toolbar) toolbar.classList.toggle('hidden', picked === 0);
      const master = document.getElementById('torbox-select-all');
      if (master) {
        master.checked = picked > 0 && picked === total;
        master.indeterminate = picked > 0 && picked < total;
      }
    }

    function toggleSelectAllTorbox(checked) {
      document.querySelectorAll('.torbox-item-cb').forEach(cb => { cb.checked = checked; });
      updateTorboxSelection();
    }

    async function deleteSelectedTorbox() {
      const boxes = Array.from(document.querySelectorAll('.torbox-item-cb:checked'));
      if (boxes.length === 0) { showToast('Ch\u01b0a ch\u1ecdn m\u1ee5c n\u00e0o.', 'info'); return; }
      if (!confirm(`X\u00f3a ${boxes.length} torrent kh\u1ecfi TorBox? Thao t\u00e1c n\u00e0y kh\u00f4ng ho\u00e0n t\u00e1c \u0111\u01b0\u1ee3c.`)) return;
      let ok = 0, fail = 0;
      for (const cb of boxes) {
        try {
          const res = await fetch('/api/torbox/delete', {
            method: 'POST', headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({ id: parseInt(cb.value, 10) })
          });
          ((await res.json()).success) ? ok++ : fail++;
        } catch (e) { fail++; }
      }
      showToast(`\u0110\u00e3 x\u00f3a ${ok} torrent${fail ? `, ${fail} l\u1ed7i` : ''}.`, fail ? 'error' : 'success');
      fetchTorrents();
    }

    // The episode rows call openVideoPlayer(); the real implementation is playEpisode().
    function openVideoPlayer(show, season, filename) {
      return playEpisode(decodeURIComponent(show), decodeURIComponent(season), decodeURIComponent(filename));
    }

    async function cancelSyncJob(jobId) {
      try {
        const res = await fetch('/api/download/cancel', {
          method: 'POST', headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ job_id: jobId })
        });
        const data = await res.json();
        showToast(data.message || (data.success ? '\u0110\u00e3 h\u1ee7y.' : '\u0110\u00e3 k\u1ebft th\u00fac.'), data.success ? 'success' : 'info');
        setTimeout(fetchTorrents, 600);
      } catch (e) { showToast(`\u274c ${e}`, 'error'); }
    }


    async function startQueuedDownload(queuedId) {
      try {
        const res = await fetch('/api/torbox/control_queued', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({id: queuedId, operation: 'start'})
        });
        const data = await res.json();
        if (data.success) {
          showToast('🚀 Đã kích hoạt tải torrent trong TorBox Cloud!', 'success');
          fetchData();
        } else {
          showToast('Lỗi: ' + (data.error || data.detail), 'error');
        }
      } catch (e) {
        showToast('Lỗi kết nối: ' + e, 'error');
      }
    }

    async function deleteQueuedDownload(queuedId) {
      if (!confirm(`Bạn có chắc muốn xóa download #${queuedId} khỏi hàng đợi?`)) return;
      try {
        const res = await fetch('/api/torbox/control_queued', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({id: queuedId, operation: 'delete'})
        });
        const data = await res.json();
        if (data.success) {
          showToast('Đã xóa khỏi hàng đợi thành công!', 'success');
          fetchData();
        } else {
          showToast('Lỗi: ' + (data.error || data.detail), 'error');
        }
      } catch (e) {
        showToast('Lỗi kết nối: ' + e, 'error');
      }
    }

    async function downloadTorboxTorrent(id, name) {
      try {
        const res = await fetch(`/api/torbox/download_link?id=${id}`);
        const data = await res.json();
        if (data.data) {
          const downloadUrl = data.data;
          window.open(downloadUrl, '_blank');
          showToast('⚡ Đã mở link tải torrent trực tiếp về máy!', 'success');
        } else {
          showToast('Chưa lấy được link: ' + (data.error || data.detail || 'Torrent đang xử lý'), 'warning');
        }
      } catch (e) {
        showToast('Lỗi kết nối TorBox API: ' + e, 'error');
      }
    }

    async function copyTorboxDownloadLink(id) {
      try {
        const res = await fetch(`/api/torbox/download_link?id=${id}`);
        const data = await res.json();
        if (data.data) {
          navigator.clipboard.writeText(data.data);
          showToast('📋 Đã copy Direct Link tốc độ cao vào bộ nhớ tạm!', 'success');
        } else {
          showToast('Chưa có direct link: ' + (data.error || data.detail), 'warning');
        }
      } catch (e) {
        showToast('Lỗi kết nối: ' + e, 'error');
      }
    }

    async function submitMagnet() {
      const magnet = document.getElementById('magnet-input').value.trim();
      if (!magnet) return showToast('Vui lòng dán link magnet!', 'warning');
      try {
        const res = await fetch('/api/torbox/add', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({magnet})
        });
        const d = await res.json();
        if (d.success) {
          showToast('✓ Đã thêm magnet thành công vào TorBox Cloud!', 'success');
          closeModal('modal-add-magnet');
          document.getElementById('magnet-input').value = '';
          fetchData();
        } else {
          showToast('Lỗi: ' + (d.error || d.detail), 'error');
        }
      } catch (e) {
        showToast('Lỗi kết nối: ' + e, 'error');
      }
    }

    async function deleteTorrent(id) {
      if (!confirm(`Bạn có chắc muốn xóa torrent #${id}?`)) return;
      try {
        await fetch('/api/torbox/delete', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({id})
        });
        fetchData();
        showToast(`Đã xóa torrent #${id} thành công!`, 'success');
      } catch (e) {
        console.error(e);
      }
    }


export {
  queueSyncJob,
  syncSingleTorrent,
  syncAllTorrents,
  syncSelectedTorbox,
  updateTorboxSelection,
  toggleSelectAllTorbox,
  deleteSelectedTorbox,
  cancelSyncJob,
  startQueuedDownload,
  deleteQueuedDownload,
  downloadTorboxTorrent,
  copyTorboxDownloadLink,
  submitMagnet,
  deleteTorrent
};
