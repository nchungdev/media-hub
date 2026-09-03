#!/usr/bin/env bash
# ==============================================================================
# Installer for Antigravity Media Hub Standalone Web App, CLI & Desktop App
# ==============================================================================

set -e

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
BIN_SOURCE="$DIR/bin/media-hub"
TARGET_DIR="${HOME}/.local/bin"
APP_TARGET="${HOME}/Applications"

export PATH="/Users/chungnh/.local/share/fnm/node-versions/v24.20.0/installation/bin:$HOME/.local/bin:$PATH"

mkdir -p "$TARGET_DIR"
mkdir -p "$APP_TARGET"

echo "🪐 Đang cài đặt Antigravity Media Hub Standalone CLI & Desktop App..."

# 1. Symlink CLI executable
ln -sf "$BIN_SOURCE" "$TARGET_DIR/media-hub"
chmod +x "$BIN_SOURCE"
echo "✅ Đã tạo liên kết CLI tại: $TARGET_DIR/media-hub"

# 2. Setup Native Desktop App (Tauri & Rust 12MB bundle)
TAURI_APP="$DIR/src-tauri/target/release/bundle/macos/Media Hub.app"
if [ -d "$TAURI_APP" ]; then
  cp -R "$TAURI_APP" "$APP_TARGET/"
  echo "✅ Đã cài đặt ứng dụng Native Desktop (Rust & Tauri 2.0) vào: $APP_TARGET/Media Hub.app (12MB)"
fi

# 3. Check PATH
if [[ ":$PATH:" != *":$TARGET_DIR:"* ]]; then
  echo "⚠️ Lưu ý: $TARGET_DIR chưa có trong PATH của shell hiện tại."
  echo "👉 Hãy thêm dòng sau vào ~/.zshrc hoặc ~/.bashrc:"
  echo '   export PATH="$HOME/.local/bin:$PATH"'
fi

echo ""
echo "🎉 Cài đặt hoàn tất! Các cách sử dụng Media Hub:"
echo "   1. 🖥️ Mở Ứng Dụng Desktop : Mở 'Media Hub' từ Launchpad / Spotlight / Dock, hoặc gõ 'media-hub app'"
echo "   2. 🌐 Mở Web Dashboard   : Gõ 'media-hub open' hoặc 'media-hub start --open'"
echo "   3. ⚡ Quản Lý Qua CLI     : 'media-hub status', 'media-hub logs -f', 'media-hub restart'"
echo ""
