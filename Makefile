.PHONY: help dev tauri-dev server server-rust cli build clean

help:
	@echo "Antigravity Media Hub - Management Commands:"
	@echo "  make dev          - Run Desktop App via Cargo (default)"
	@echo "  make server       - Run Python Backend Server (http://127.0.0.1:8888)"
	@echo "  make server-rust  - Run Headless Rust Axum Server"
	@echo "  make tauri-dev    - Run Tauri Dev with Hot Reload"
	@echo "  make cli          - Run CLI launcher (bin/media-hub)"
	@echo "  make build        - Build release binary for desktop"

dev:
	cargo run --manifest-path src-tauri/Cargo.toml

server:
	python3 backend/server.py

server-rust:
	cargo run --manifest-path src-tauri/Cargo.toml -- --server

tauri-dev:
	cargo tauri dev

cli:
	./bin/media-hub start --foreground

build:
	cargo build --release --manifest-path src-tauri/Cargo.toml

clean:
	cargo clean --manifest-path src-tauri/Cargo.toml
