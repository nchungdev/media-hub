// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--server" || a == "-s") {
        let port = std::env::var("MEDIA_HUB_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8888);
        println!("🚀 Starting Media Hub Pure Rust Server on http://0.0.0.0:{}...", port);
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                if let Err(e) = media_hub_lib::run_server_headless(port).await {
                    eprintln!("Server error: {}", e);
                }
            });
        return;
    }
    media_hub_lib::run();
}

