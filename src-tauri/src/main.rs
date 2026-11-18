// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;

/// What the Phase 0 prototype shows: proof that the window reached Rust.
#[derive(Serialize)]
struct Probe {
    os: &'static str,
    arch: &'static str,
    user_folder: Option<String>,
    app_version: &'static str,
}

#[tauri::command]
fn probe() -> Probe {
    Probe {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        user_folder: std::env::var("USERPROFILE").ok(),
        app_version: env!("CARGO_PKG_VERSION"),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![probe])
        .run(tauri::generate_context!())
        .expect("error while running XkyHold-wd");
}
