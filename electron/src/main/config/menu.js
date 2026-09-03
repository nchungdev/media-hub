/**
 * @file menu.js
 * @description Factory for creating native application menus with macOS and cross-platform support.
 */

'use strict';

const { app, Menu } = require('electron');

/**
 * Creates and sets the application menu.
 * @returns {Menu}
 */
function createApplicationMenu() {
  const isMac = process.platform === 'darwin';

  const template = [
    ...(isMac ? [{
      label: app.name,
      submenu: [
        { role: 'about', label: 'Giới Thiệu Media Hub' },
        { type: 'separator' },
        { role: 'services' },
        { type: 'separator' },
        { role: 'hide', label: 'Ẩn Media Hub' },
        { role: 'hideOthers', label: 'Ẩn Ứng Dụng Khác' },
        { role: 'unhide', label: 'Hiện Tất Cả' },
        { type: 'separator' },
        { role: 'quit', label: 'Thoát Media Hub' }
      ]
    }] : []),
    {
      label: 'Chỉnh Sửa',
      submenu: [
        { role: 'undo', label: 'Hoàn tác' },
        { role: 'redo', label: 'Làm lại' },
        { type: 'separator' },
        { role: 'cut', label: 'Cắt' },
        { role: 'copy', label: 'Sao chép' },
        { role: 'paste', label: 'Dán' },
        { role: 'selectAll', label: 'Chọn tất cả' }
      ]
    },
    {
      label: 'Hiển Thị',
      submenu: [
        { role: 'reload', label: 'Tải Lại Giao Diện' },
        { role: 'forceReload', label: 'Làm Mới Toàn Bộ' },
        { role: 'toggleDevTools', label: 'Bật Developer Tools' },
        { type: 'separator' },
        { role: 'resetZoom', label: 'Cỡ Chữ Gốc' },
        { role: 'zoomIn', label: 'Phóng To' },
        { role: 'zoomOut', label: 'Thu Nhỏ' },
        { type: 'separator' },
        { role: 'togglefullscreen', label: 'Toàn Màn Hình' }
      ]
    },
    {
      label: 'Cửa Sổ',
      submenu: [
        { role: 'minimize', label: 'Thu Nhỏ' },
        { role: 'zoom', label: 'Phóng To Cửa Sổ' },
        ...(isMac ? [
          { type: 'separator' },
          { role: 'front', label: 'Đưa Ra Trước' }
        ] : [
          { role: 'close', label: 'Đóng' }
        ])
      ]
    }
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
  return menu;
}

module.exports = { createApplicationMenu };
