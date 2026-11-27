//! Grace's desktop app: one window showing the UI, with Grace's local API
//! running in-process on 127.0.0.1 (spec §7.2). The window learns the API's
//! address and token through the `api_info` command.

// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use grace_core::api::{self, ApiOptions};
use grace_core::discover::DiscoveryEnv;
use serde::Serialize;
use tauri::Manager;

#[derive(Clone, Serialize)]
struct ApiInfo {
    base_url: String,
    token: String,
}

#[tauri::command]
fn api_info(info: tauri::State<'_, ApiInfo>) -> ApiInfo {
    info.inner().clone()
}

/// The page origins the desktop window can have (Windows, macOS/Linux, `tauri dev`).
const WINDOW_ORIGINS: [&str; 4] = [
    "http://tauri.localhost",
    "https://tauri.localhost",
    "tauri://localhost",
    "http://localhost:5173",
];

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            let token = api::random_token();
            let discovery = DiscoveryEnv::from_system();
            let options = ApiOptions {
                grace_home: discovery.grace_home.clone(),
                token: token.clone(),
                allowed_origins: WINDOW_ORIGINS.iter().map(|o| o.to_string()).collect(),
                discovery,
                ui: None,
            };
            tauri::async_runtime::spawn(async move {
                match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => {
                        let router = api::router(options);
                        if let Err(err) = api::run(listener, router, std::future::pending()).await {
                            eprintln!("Grace's engine stopped: {err}");
                        }
                    }
                    Err(err) => eprintln!("Grace's engine couldn't start: {err}"),
                }
            });
            app.manage(ApiInfo {
                base_url: format!("http://127.0.0.1:{port}"),
                token,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api_info])
        .run(tauri::generate_context!())
        .expect("Grace couldn't start");
}
