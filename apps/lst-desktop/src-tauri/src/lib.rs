use anyhow::Result;
use lst_cli::storage::{list_lists, list_notes, markdown::load_list};
use lst_cli::models::List;
use serde::{Deserialize, Serialize};
use specta_typescript::Typescript;
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

#[tauri::command]
#[specta::specta]
fn get_lists() -> Result<Vec<String>, String> {
    list_lists().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_notes() -> Result<Vec<String>, String> {
    list_notes().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_list(name: String) -> Result<List, String> {
    load_list(&name).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        // Then register them (separated by a comma)
        .commands(collect_commands![get_lists, get_notes, get_list]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let tray = TrayIconBuilder::new().build(app)?;
            let window = app.get_webview_window("main").unwrap();

            // #[cfg(target_os = "macos")]
            // window_vibrancy::apply_vibrancy(
            //     &window,
            //     window_vibrancy::NSVisualEffectMaterial::HudWindow,
            //     None,
            //     Some(5.0),
            // )
            // .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            #[cfg(target_os = "windows")]
            window_vibrancy::apply_blur(&window, Some((18, 18, 18, 125)))
                .expect("Unsupported platform! 'apply_blur' is only supported on Windows");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_lists, get_notes, get_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
