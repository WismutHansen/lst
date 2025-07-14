use anyhow::Result;
use lst_cli::config::{get_config, UiConfig};
use lst_cli::models::List;
use serde::{Deserialize, Serialize};
use specta::Type;

mod crypto;
mod database;
mod sync;
mod sync_db;
use database::Database;
use specta_typescript::Typescript;
use tauri::Manager;
use tauri_plugin_opener;
use tauri_specta::{collect_commands, Builder};

mod command_server;
mod theme;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub created: Option<String>,
    pub file_path: String,
}

#[tauri::command]
#[specta::specta]
fn get_lists(db: tauri::State<'_, Database>) -> Result<Vec<String>, String> {
    db.list_titles().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_notes(db: tauri::State<'_, Database>) -> Result<Vec<String>, String> {
    db.list_note_titles().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_note(name: String, db: tauri::State<'_, Database>) -> Result<Note, String> {
    db.load_note(&name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn create_note_cmd(title: String, db: tauri::State<'_, Database>) -> Result<Note, String> {
    db.create_note(&title).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn save_note(note: Note, db: tauri::State<'_, Database>) -> Result<(), String> {
    db.save_note(&note).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn delete_note_cmd(name: String, db: tauri::State<'_, Database>) -> Result<(), String> {
    db.delete_note(&name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_list(name: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.load_list(&name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn create_list(title: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.create_list(&title).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn add_item(list: String, text: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.add_item(&list, &text).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn toggle_item(
    list: String,
    target: String,
    db: tauri::State<'_, Database>,
) -> Result<List, String> {
    db.toggle_item(&list, &target).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn remove_item(
    list: String,
    target: String,
    db: tauri::State<'_, Database>,
) -> Result<List, String> {
    db.remove_item(&list, &target).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_ui_config() -> Result<UiConfig, String> {
    Ok(get_config().ui.clone())
}

#[tauri::command]
#[specta::specta]
fn edit_item(
    list: String,
    target: String,
    text: String,
    db: tauri::State<'_, Database>,
) -> Result<List, String> {
    db.edit_item(&list, &target, &text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn reorder_item(
    list: String,
    target: String,
    new_index: u32,
    db: tauri::State<'_, Database>,
) -> Result<List, String> {
    db.reorder_item(&list, &target, new_index as usize)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn save_list(list: List, db: tauri::State<'_, Database>) -> Result<(), String> {
    db.save_list(&list).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        // Then register them (separated by a comma)
        .commands(collect_commands![
            get_lists,
            get_notes,
            get_note,
            create_note_cmd,
            save_note,
            delete_note_cmd,
            get_list,
            create_list,
            add_item,
            toggle_item,
            edit_item,
            remove_item,
            reorder_item,
            save_list,
            get_ui_config
        ]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let db = Database::new(&handle)?;
            app.manage(db);
            let _window = app.get_webview_window("main").unwrap();

            command_server::start_command_server(app.handle().clone());
            theme::broadcast_theme(&app.handle()).ok();

            // spawn background sync task
            let config = get_config().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(mut mgr) = sync::SyncManager::new(config).await {
                    loop {
                        if let Err(e) = mgr.periodic_sync().await {
                            eprintln!("sync error: {e}");
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    }
                }
            });

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
        .invoke_handler(tauri::generate_handler![
            get_lists,
            get_notes,
            get_note,
            create_note_cmd,
            save_note,
            delete_note_cmd,
            get_list,
            create_list,
            add_item,
            toggle_item,
            edit_item,
            remove_item,
            reorder_item,
            save_list,
            get_ui_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
