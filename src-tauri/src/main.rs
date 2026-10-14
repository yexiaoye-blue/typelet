#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use typelet_typing::{statistics, update_practice, Practice, Statistics};

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
        .invoke_handler(tauri::generate_handler![
            typing_snapshot,
            fonts::list_system_fonts
        ])
        .run(tauri::generate_context!())
        .expect("error while running Typelet");
}
