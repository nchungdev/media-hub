# 🪐 Antigravity Media Hub

<div align="center">

[![Release](https://img.shields.io/github/v/release/nchungdev/media-hub?color=blue&style=flat-square)](https://github.com/nchungdev/media-hub/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/nchungdev/media-hub/release.yml?branch=main&style=flat-square)](https://github.com/nchungdev/media-hub/actions)
[![Platform](https://img.shields.io/badge/Platform-macOS%20(Apple%20Silicon%20%26%20Intel)-black?style=flat-square&logo=apple)](https://github.com/nchungdev/media-hub/releases)
[![Electron](https://img.shields.io/badge/Electron-34.0.0-47848F?style=flat-square&logo=electron)](https://electronjs.org)
[![Python](https://img.shields.io/badge/Python-3.9%2B-3776AB?style=flat-square&logo=python)](https://python.org)
[![Skills](https://img.shields.io/badge/Agent%20Skills-nchungdev%2Fagent--skills-8A2BE2?style=flat-square)](https://github.com/nchungdev/agent-skills)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

**Antigravity Media Hub** là Ứng dụng Desktop Native (macOS) & Trạm Chỉ Huy Media AI Toàn Diện, kết nối trực tiếp với **Antigravity AI Agent (`agy` / `agy2`)** để tự động hóa trọn gói: Tải phim đa nguồn (TorBox/Aria2/DDL), Quản lý Thư viện Media kiểu Plex, Dịch thuật Phụ đề Song ngữ chuyên sâu, Bóc tách Subtitle và Đồng bộ Đa Đích (NAS Storage & Google Drive).

[**Tải Bản Cài Đặt (.dmg)**](https://github.com/nchungdev/media-hub/releases) • [**Hệ Sinh Thái Agent Skills**](https://github.com/nchungdev/agent-skills) • [**Tài Liệu Hướng Dẫn**](#-hướng-dẫn-sử-dụng)

</div>

---

## 🌟 Tính Năng Nổi Bật (Key Features)

* 🖥️ **Ứng Dụng Desktop Native macOS & Web Dashboard**: Thiết kế giao diện Dark Cyberpunk sang trọng với thanh tiêu đề `hiddenInset` chuẩn macOS, hỗ trợ thu phóng và vận hành mượt mà cả dưới dạng App Desktop độc lập lẫn Web UI trên cổng `8888`.
* 📁 **Tách Biệt Thư Mục Ứng Dụng & Workspace (Decoupled Workspace Architecture)**: App chạy độc lập, cho phép người dùng chọn bất kỳ thư mục/ổ đĩa nào làm Media Workspace qua hộp thoại macOS Native Folder Chooser.
* 🎬 **Subtitle Studio Toàn Màn Hình**: Theo dõi tiến độ dịch thuật từng tập phim trực quan với Progress Bar, KPI Cards, nút bấm dịch 1-click chia batch ngầm an toàn và cơ chế khóa nút chống spam token.
* 🤖 **Tự Động Kích Hoạt Antigravity AI CLI (`agy` / `agy2`)**: Tự động spawn tiến trình CLI ngầm, điều phối session thông minh theo `media-id`, tận dụng tối đa Context Caching và tự động fallback luân chuyển giữa Secondary/Primary profile khi chạm quota.
* 📟 **Live CLI Terminal Console Realtime**: Màn hình Console chuyên dụng stream thời gian thực toàn bộ log stdout/stderr của AI Agent, hỗ trợ tìm kiếm/lọc log, auto-scroll và nút **Dừng / Chạy CLI (`⏹ Dừng CLI` / `▶️ Chạy CLI`)** tức thời.
* 📊 **Báo Cáo Token Usage & Ước Tính Chi Phí AI**: Thống kê chi tiết Input/Output/Thinking tokens, số lượt tương tác (Turns) và ước tính chi phí ($ USD) cho từng bộ phim với tính năng xóa cache 1-click.
* 📥 **Bộ Tải Đa Nguồn 4 Engines**: Tích hợp TorBox Cloud Debrid (kéo torrent tốc độ cao không tốn mạng nhà), Aria2c P2P Client, Direct Download Link (DDL đa luồng) và yt-dlp Stream Extractor.
* ☁️ **Đồng Bộ Đa Đích & Tự Động Giải Phóng Bộ Đệm (Auto-Purge)**: Đẩy song song lên NAS Storage qua SSH/SFTP và Google Drive qua Rclone, tự động xóa sạch file đệm trên máy ngay sau khi truyền tải an toàn.

---

## 🏛️ Kiến Trúc Hệ Sinh Thái & Liên Kết Kho Lưu Trữ

Media Hub hoạt động song hành cùng hệ sinh thái **Agent Skills**:

```
┌─────────────────────────────────────────────────────────────┐
│             🪐 ANTIGRAVITY MEDIA HUB (This Repo)             │
│        Desktop Native App (Electron) + Host Web Server      │
│  (Subtitle Studio, Live CLI Console, Token Usage, TorBox)   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Tự động kích hoạt & Dispatch
                               ▼
┌─────────────────────────────────────────────────────────────┐
│             🤖 AGENT SKILLS ECOSYSTEM REPOSITORY            │
│          👉 https://github.com/nchungdev/agent-skills        │
├──────────────────────────────┬──────────────────────────────┤
│  🚀 Core Pipeline Skills     │  🧰 Media Toolbox Skills     │
│  • media-downloader          │  • translate-subtitle        │
│  • cloud-librarian           │  • subtitle-extractor        │
│  • media-sync                │  • sub-to-webvtt             │
│                              │  • tmdb-lookup               │
│                              │  • media-collector           │
└──────────────────────────────┴──────────────────────────────┘
```

---

## 📥 Cài Đặt (Installation)

### Cách 1: Tải Bản Cài Đặt Desktop App macOS (Khuyên dùng)
1. Vào trang [**Releases**](https://github.com/nchungdev/media-hub/releases) và tải file **`Media Hub-x.x.x-arm64.dmg`** (dành cho Apple Silicon M1/M2/M3/M4) hoặc **`x64.dmg`** (Intel).
2. Mở file `.dmg` và kéo **Media Hub.app** vào thư mục `Applications`.
3. Mở app, chọn Thư mục Làm việc Media của bạn và bắt đầu sử dụng!

### Cách 2: Cài Đặt Lệnh CLI Toàn Cục (`media-hub`)
Nếu bạn muốn điều khiển server từ Terminal, chạy script cài đặt:

```bash
git clone https://github.com/nchungdev/media-hub.git
cd media-hub
./install.sh
```

---

## 🛠️ Các Lệnh Điều Khiển CLI

| Lệnh | Chức Năng |
| :--- | :--- |
| `media-hub app` | Mở ứng dụng Desktop Native Electron (`Media Hub.app`) |
| `media-hub start` | Khởi chạy máy chủ Dashboard ngầm tại cổng 8888 |
| `media-hub start --open` | Khởi chạy máy chủ và tự động mở Web Dashboard trên trình duyệt |
| `media-hub status` | Kiểm tra trạng thái máy chủ, PID, Port và đường dẫn Workspace |
| `media-hub open` | Mở Web Dashboard trên trình duyệt (`http://127.0.0.1:8888`) |
| `media-hub logs -f` | Theo dõi log máy chủ và live output của AI Agent |
| `media-hub stop` | Dừng máy chủ chạy ngầm |
| `media-hub restart` | Khởi động lại máy chủ |

---

## 🚀 Tự Động Hóa CI/CD Release (GitHub Actions)

Repository này được tích hợp sẵn quy trình CI/CD tự động build và xuất bản bản cài đặt Desktop macOS qua GitHub Actions:

```bash
# Tạo tag phiên bản mới và đẩy lên GitHub
git tag v2.4.0
git push origin v2.4.0
```

GitHub Actions sẽ tự động:
1. Setup môi trường macOS Apple Silicon (`macos-14`) và Intel.
2. Cài đặt dependency và đóng gói `.dmg`, `.blockmap`.
3. Tự động tạo bản **GitHub Release** và đính kèm đầy đủ file cài đặt.

---

## 📄 Giấy Phép (License)

Phát hành theo giấy phép [MIT](LICENSE) — © 2026 Chung Nguyen Hoai ([@nchungdev](https://github.com/nchungdev)).
