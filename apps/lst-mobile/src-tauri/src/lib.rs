use anyhow::Result;
use lst_cli::config::{get_config, UiConfig};
use lst_cli::models::List;
use serde::{Deserialize, Serialize};
use specta::Type;

mod auth;
mod crypto;
mod database;
mod sync;
mod sync_db;
mod sync_status;
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

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SyncConfig {
    pub server_url: String,
    pub email: String,
    pub device_id: String,
    pub sync_enabled: bool,
    pub sync_interval: u32,
    pub encryption_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SyncStatus {
    pub connected: bool,
    pub last_sync: Option<String>,
    pub pending_changes: u32,
    pub error: Option<String>,
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
fn add_item(list: String, text: String, category: Option<String>, db: tauri::State<'_, Database>) -> Result<List, String> {
    match category {
        Some(cat) => db.add_item_to_category(&list, &text, Some(&cat)).map_err(|e| e.to_string()),
        None => db.add_item(&list, &text).map_err(|e| e.to_string()),
    }
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

#[tauri::command]
#[specta::specta]
fn create_category(list_name: String, category_name: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.create_category(&list_name, &category_name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn move_item_to_category(list_name: String, item_anchor: String, category_name: Option<String>, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.move_item_to_category(&list_name, &item_anchor, category_name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn delete_category(list_name: String, category_name: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.delete_category(&list_name, &category_name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_categories(list_name: String, db: tauri::State<'_, Database>) -> Result<Vec<String>, String> {
    db.get_categories(&list_name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn rename_category(list_name: String, old_name: String, new_name: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.rename_category(&list_name, &old_name, &new_name).map_err(|e| e.to_string())
}

// Sync-related commands
#[tauri::command]
#[specta::specta]
fn get_sync_config() -> Result<SyncConfig, String> {
    match auth::get_sync_config() {
        Ok((server_url, email, device_id, sync_enabled, sync_interval)) => {
            Ok(SyncConfig {
                server_url,
                email,
                device_id,
                sync_enabled,
                sync_interval,
                encryption_enabled: true,
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
#[specta::specta]
fn save_sync_config(config: SyncConfig) -> Result<(), String> {
    auth::update_sync_config(
        config.server_url,
        config.email,
        config.device_id,
        config.sync_enabled,
        config.sync_interval,
    ).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_sync_status() -> Result<SyncStatus, String> {
    match sync_status::get_sync_status() {
        Ok(status) => Ok(SyncStatus {
            connected: status.connected,
            last_sync: status.last_sync,
            pending_changes: status.pending_changes,
            error: status.error,
        }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
#[specta::specta]
async fn request_auth_token(email: String, server_url: String, password: Option<String>) -> Result<String, String> {
    auth::request_auth_token(email, server_url, password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn verify_auth_token(email: String, token: String) -> Result<String, String> {
    // We need the server URL for verification, let's get it from config
    let config = get_config();
    let server_url = config.syncd
        .as_ref()
        .and_then(|s| s.url.as_ref())
        .cloned()
        .unwrap_or_default();
    
    if server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }
    
    auth::verify_auth_token(email, token, server_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn toggle_sync(enabled: bool) -> Result<(), String> {
    println!("Toggling sync: {}", enabled);
    
    if enabled {
        // Mark sync as attempting to connect
        sync_status::mark_sync_disconnected("Connecting...".to_string())
            .map_err(|e| e.to_string())?;
    } else {
        // Mark sync as disabled
        sync_status::mark_sync_disconnected("Sync disabled".to_string())
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
#[specta::specta]
async fn test_sync_connection() -> Result<String, String> {
    let config = get_config();
    
    // Check if sync is configured
    let server_url = config.syncd
        .as_ref()
        .and_then(|s| s.url.as_ref())
        .ok_or("Server URL not configured")?;
    
    if !config.is_jwt_valid() {
        return Err("No valid JWT token. Please authenticate first.".to_string());
    }
    
    // Test basic HTTP connectivity
    let base_url = server_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .replace("/api/sync", "");
    
    let client = reqwest::Client::new();
    match client.get(&format!("{}/api/health", base_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                Ok("Server connection successful!".to_string())
            } else {
                Err(format!("Server returned status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to connect to server: {}", e)),
    }
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
            get_ui_config,
            create_category,
            move_item_to_category,
            delete_category,
            get_categories,
            rename_category,
            get_sync_config,
            save_sync_config,
            get_sync_status,
            request_auth_token,
            verify_auth_token,
            toggle_sync,
            test_sync_connection
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

            // Start command server and sync on desktop platforms
            #[cfg(not(mobile))]
            {
                command_server::start_command_server(app.handle().clone());
                theme::broadcast_theme(&app.handle()).ok();
            }

            // Start sync service on all platforms (including mobile)
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

            // Apply vibrancy effects only on desktop platforms
            #[cfg(all(target_os = "macos", not(mobile)))]
            window_vibrancy::apply_vibrancy(
                &_window,
                window_vibrancy::NSVisualEffectMaterial::HudWindow,
                None,
                Some(5.0),
            )
            .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            #[cfg(all(target_os = "windows", not(mobile)))]
            window_vibrancy::apply_blur(&_window, Some((18, 18, 18, 125)))
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
            get_ui_config,
            create_category,
            move_item_to_category,
            delete_category,
            get_categories,
            rename_category,
            get_sync_config,
            save_sync_config,
            get_sync_status,
            request_auth_token,
            verify_auth_token,
            toggle_sync,
            test_sync_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
