#!/usr/bin/env bash
# ==============================================================================
# Antigravity Media Hub - One-Click Launcher
# Usage:
#   bash launch_dashboard.sh          # Chế độ nội bộ (Localhost:8888)
#   bash launch_dashboard.sh --tunnel # Mở đường truyền TryCloudflare online
# ==============================================================================

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$DIR/launcher.py" "$@"
