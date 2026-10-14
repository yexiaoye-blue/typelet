#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use typelet_typing::{statistics, update_practice, Practice, Statistics};

mod agents;
mod fonts;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    practice: Practice,
    stats: Statistics,
}

#[tauri::command]
fn typing_snapshot(
    previous: Practice,
    value: String,
    target: String,
    milliseconds: u64,
) -> Snapshot {
    let practice = update_practice(&previous, &value, &target);
    let stats = statistics(&practice, &target, milliseconds);
    Snapshot { practice, stats }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            typing_snapshot,
            fonts::list_system_fonts,
            agents::get_agent_config,
            agents::set_codex_agent_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running Typelet");
}
