const { app, BrowserWindow, Menu, shell, dialog } = require('electron');
const path = require('path');
const http = require('http');
const { spawn } = require('child_process');

const fs = require('fs');

let mainWindow = null;
let serverProcess = null;
const SERVER_PORT = process.env.PORT || 8888;
const SERVER_URL = `http://127.0.0.1:${SERVER_PORT}`;

function findPythonBinary() {
  const candidates = [
    '/opt/homebrew/bin/python3',
    '/usr/local/bin/python3',
    '/usr/bin/python3',
    '/Library/Developer/CommandLineTools/usr/bin/python3',
    'python3'
  ];
  for (const c of candidates) {
    try {
      if (c.startsWith('/') && fs.existsSync(c)) {
        return c;
      }
    } catch (e) {}
  }
  return 'python3';
}

function getServerPaths() {
  const isPackaged = app.isPackaged;
  let appDir = path.resolve(__dirname, '..');
  let scriptPath = path.join(appDir, 'scripts', 'server.py');

  if (isPackaged || !fs.existsSync(scriptPath)) {
    const resourceDir = process.resourcesPath || path.join(__dirname, '..');
    const candScript = path.join(resourceDir, 'scripts', 'server.py');
    if (fs.existsSync(candScript)) {
      appDir = resourceDir;
      scriptPath = candScript;
    }
  }
  return { appDir, scriptPath };
}

function checkServerReady(retries = 25, interval = 300) {
  return new Promise((resolve) => {
    let attempt = 0;
    const tryPing = () => {
      attempt++;
      const req = http.get(`${SERVER_URL}/api/settings`, (res) => {
        if (res.statusCode === 200) {
          resolve(true);
        } else if (attempt < retries) {
          setTimeout(tryPing, interval);
        } else {
          resolve(false);
        }
      });
      req.on('error', () => {
        if (attempt < retries) {
          setTimeout(tryPing, interval);
        } else {
          resolve(false);
        }
      });
      req.setTimeout(500, () => {
        req.destroy();
        if (attempt < retries) {
          setTimeout(tryPing, interval);
        } else {
          resolve(false);
        }
      });
    };
    tryPing();
  });
}

function startBackendServer() {
  return new Promise(async (resolve) => {
    const isAlreadyRunning = await checkServerReady(2, 200);
    if (isAlreadyRunning) {
      console.log('⚡ Backend server already running on port', SERVER_PORT);
      return resolve(true);
    }

    const pythonBin = findPythonBinary();
    const { appDir, scriptPath } = getServerPaths();

    console.log('🚀 Spawning Python Backend Server:');
    console.log('   - Python Binary:', pythonBin);
    console.log('   - Script Path:  ', scriptPath);
    console.log('   - App Directory:', appDir);

    const extraPath = [
      '/opt/homebrew/bin',
      '/opt/homebrew/sbin',
      '/usr/local/bin',
      '/usr/bin',
      '/bin',
      '/usr/sbin',
      '/sbin',
      path.join(process.env.HOME || '', '.local', 'bin')
    ].join(':');

    const env = Object.assign({}, process.env, {
      PORT: String(SERVER_PORT),
      PYTHONUNBUFFERED: '1',
      PATH: extraPath + (process.env.PATH ? ':' + process.env.PATH : '')
    });

    const logDir = path.join(process.env.HOME || '', '.media-hub', '.logs');
    try {
      fs.mkdirSync(logDir, { recursive: true });
    } catch (e) {}

    const outLogPath = path.join(logDir, 'server.log');
    const outLogFd = fs.openSync(outLogPath, 'a');

    serverProcess = spawn(pythonBin, [scriptPath], {
      cwd: appDir,
      env: env,
      stdio: ['ignore', outLogFd, outLogFd],
      detached: true
    });
    serverProcess.unref();

    console.log(`[PythonServer] Spawned detached daemon (PID: ${serverProcess.pid}), logging to ${outLogPath}`);

    const ready = await checkServerReady(35, 400);
    resolve(ready);
  });
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 920,
    minWidth: 1024,
    minHeight: 700,
    backgroundColor: '#09090b',
    titleBarStyle: 'hiddenInset',
    trafficLightPosition: { x: 18, y: 18 },
    show: false,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: false
    }
  });

  mainWindow.loadURL(SERVER_URL);

  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });

  // Open external links in default browser
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http://') || url.startsWith('https://')) {
      if (!url.includes(`127.0.0.1:${SERVER_PORT}`)) {
        shell.openExternal(url);
        return { action: 'deny' };
      }
    }
    return { action: 'allow' };
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

function createMenu() {
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
}

app.whenReady().then(async () => {
  createMenu();
  const serverStarted = await startBackendServer();
  if (!serverStarted) {
    dialog.showErrorBox(
      'Lỗi Khởi Chạy Máy Chủ',
      `Không thể kết nối với máy chủ Media Hub tại ${SERVER_URL}. Vui lòng kiểm tra Python 3.`
    );
  } else {
    // Proactively ensure / attach to the independent CLI Agent service
    try {
      const http = require('http');
      const req = http.request(`${SERVER_URL}/api/agent/service/ensure`, { method: 'POST' }, (res) => {
        console.log(`[CLI Service] Auto ensure/attach response status: ${res.statusCode}`);
      });
      req.on('error', () => {});
      req.end();
    } catch (e) {}
  }
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('will-quit', () => {
  // Server is kept alive as a background daemon so running CLI jobs
  // are NOT interrupted when the Electron window is closed or restarted.
  // Use `media-hub stop` to explicitly stop the server.
  if (serverProcess) {
    console.log('💤 Server process detached — will keep running in background.');
    serverProcess.unref();
  }
});
