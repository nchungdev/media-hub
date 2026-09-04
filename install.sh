#!/usr/bin/env bash
# ==============================================================================
# Installer for Antigravity Media Hub Standalone Web App, CLI & Desktop App
# ==============================================================================

set -e

DIR="$( cd -P "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
BIN_SOURCE="$DIR/bin/media-hub"
TARGET_DIR="${HOME}/.local/bin"
APP_TARGET="${HOME}/Applications"

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

# 3. Check & configure PATH
SHELL_NAME="$(basename "${SHELL:-bash}")"
case "$SHELL_NAME" in
  zsh)
    SHELL_RC="${ZDOTDIR:-$HOME}/.zshrc"
    ENV_CMD='export PATH="$HOME/.local/bin:$PATH"'
    ;;
  bash)
    if [ -f "${HOME}/.bash_profile" ]; then
      SHELL_RC="${HOME}/.bash_profile"
    else
      SHELL_RC="${HOME}/.bashrc"
    fi
    ENV_CMD='export PATH="$HOME/.local/bin:$PATH"'
    ;;
  fish)
    SHELL_RC="${HOME}/.config/fish/config.fish"
    ENV_CMD='fish_add_path "$HOME/.local/bin"'
    ;;
  *)
    SHELL_RC="${HOME}/.profile"
    ENV_CMD='export PATH="$HOME/.local/bin:$PATH"'
    ;;
esac

if [[ ":$PATH:" != *":$TARGET_DIR:"* ]]; then
  echo "⚠️ Lưu ý: $TARGET_DIR chưa có trong PATH của shell hiện tại."
  if [ -n "$SHELL_RC" ] && [ -f "$SHELL_RC" ] && grep -qs "\.local/bin" "$SHELL_RC"; then
    echo "💡 Đường dẫn đã được cấu hình trong $SHELL_RC nhưng chưa được tải."
    echo "👉 Hãy áp dụng thay đổi bằng lệnh: source $SHELL_RC"
  elif [ -n "$SHELL_RC" ] && { [ -w "$SHELL_RC" ] || [ ! -e "$SHELL_RC" ]; }; then
    echo "" >> "$SHELL_RC"
    echo "# Antigravity Media Hub CLI" >> "$SHELL_RC"
    echo "$ENV_CMD" >> "$SHELL_RC"
    echo "✅ Đã tự động cấu hình PATH vào $SHELL_RC"
    echo "👉 Hãy tải lại shell bằng lệnh: source $SHELL_RC"
  else
    echo "👉 Hãy thêm dòng sau vào $SHELL_RC:"
    echo "   $ENV_CMD"
    echo "👉 Sau đó chạy: source $SHELL_RC"
  fi
else
  echo "✅ $TARGET_DIR đã sẵn sàng trong PATH."
fi

echo ""
echo "🎉 Cài đặt hoàn tất! Các cách sử dụng Media Hub:"
echo "   1. 🖥️ Mở Ứng Dụng Desktop : Mở 'Media Hub' từ Launchpad / Spotlight / Dock, hoặc gõ 'media-hub app'"
echo "   2. 🌐 Mở Web Dashboard   : Gõ 'media-hub open' hoặc 'media-hub start --open'"
echo "   3. ⚡ Quản Lý Qua CLI     : 'media-hub status', 'media-hub logs -f', 'media-hub restart'"
echo ""
