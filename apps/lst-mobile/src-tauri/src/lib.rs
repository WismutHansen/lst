use anyhow::Result;
// Mobile app is standalone - no shared config
use lst_cli::models::List;
use serde::{Deserialize, Serialize};
use specta::Type;

mod auth;
mod crypto;
mod database;
mod mobile_config;
mod mobile_sync;
mod sync_bridge;
mod sync_db;
mod sync_status;
use database::Database;
use sync_bridge::SyncBridge;
use std::sync::Arc;
use tokio::sync::Mutex;
// use specta_typescript::Typescript; // Only needed for debug builds
use tauri::Manager;
use tauri_plugin_opener;
use tauri_specta::{collect_commands, Builder};

mod command_server;
mod theme;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
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
    println!("🔍 DEBUG: get_lists called");
    let result = db.list_titles().map_err(|e| e.to_string());
    match &result {
        Ok(lists) => println!("🔍 DEBUG: Found {} lists: {:?}", lists.len(), lists),
        Err(e) => println!("🔍 DEBUG: Error getting lists: {}", e),
    }
    result
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
async fn create_note_cmd(title: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<Note, String> {
    println!("🔍 DEBUG: create_note_cmd called with title: '{}'", title);
    db.create_note(&title, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn save_note(note: Note, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<(), String> {
    println!("🔍 DEBUG: save_note called - note: '{}'", note.title);
    db.save_note(&note, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn delete_note_cmd(name: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<(), String> {
    println!("🔍 DEBUG: delete_note_cmd called with name: '{}'", name);
    db.delete_note(&name, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_list(name: String, db: tauri::State<'_, Database>) -> Result<List, String> {
    db.load_list(&name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn create_list(title: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    println!("🔍 DEBUG: create_list command called with title: '{}'", title);
    db.create_list(&title, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn add_item(list: String, text: String, category: Option<String>, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    println!("🔍 DEBUG: add_item command called - list: '{}', text: '{}', category: {:?}", list, text, category);
    match category {
        Some(cat) => db.add_item_to_category(&list, &text, Some(&cat), Some(&app)).await.map_err(|e| e.to_string()),
        None => db.add_item(&list, &text, Some(&app)).await.map_err(|e| e.to_string()),
    }
}

#[tauri::command]
#[specta::specta]
async fn toggle_item(
    list: String,
    target: String,
    db: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> Result<List, String> {
    db.toggle_item(&list, &target, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn remove_item(
    list: String,
    target: String,
    db: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> Result<List, String> {
    db.remove_item(&list, &target, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_ui_config() -> Result<mobile_config::MobileUiConfig, String> {
    // Use default mobile config to avoid file system issues
    Ok(mobile_config::MobileUiConfig::default())
}

#[tauri::command]
#[specta::specta]
async fn edit_item(
    list: String,
    target: String,
    text: String,
    db: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> Result<List, String> {
    db.edit_item(&list, &target, &text, Some(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn reorder_item(
    list: String,
    target: String,
    new_index: u32,
    db: tauri::State<'_, Database>,
    app: tauri::AppHandle,
) -> Result<List, String> {
    db.reorder_item(&list, &target, new_index as usize, Some(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn save_list(list: List, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<(), String> {
    println!("🔍 DEBUG: save_list command called - list: '{}'", list.metadata.title);
    db.save_list(&list, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn create_category(list_name: String, category_name: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    db.create_category(&list_name, &category_name, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn move_item_to_category(list_name: String, item_anchor: String, category_name: Option<String>, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    db.move_item_to_category(&list_name, &item_anchor, category_name.as_deref(), Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn delete_category(list_name: String, category_name: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    db.delete_category(&list_name, &category_name, Some(&app)).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn get_categories(list_name: String, db: tauri::State<'_, Database>) -> Result<Vec<String>, String> {
    db.get_categories(&list_name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn rename_category(list_name: String, old_name: String, new_name: String, db: tauri::State<'_, Database>, app: tauri::AppHandle) -> Result<List, String> {
    db.rename_category(&list_name, &old_name, &new_name, Some(&app)).await.map_err(|e| e.to_string())
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
fn save_sync_config(config: SyncConfig, db: tauri::State<Database>) -> Result<(), String> {
    auth::update_sync_config_with_db(
        &db,
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
async fn verify_auth_token(email: String, token: String, server_url: String, db: tauri::State<'_, Database>) -> Result<String, String> {
    if server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }
    
    auth::verify_auth_token_with_db(email, token, server_url, &db)
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
async fn trigger_sync(_db: tauri::State<'_, Database>) -> Result<String, String> {
    let config = auth::get_current_mobile_config();
    
    if !config.has_syncd() || !config.is_jwt_valid() {
        return Err("Sync not configured or JWT expired".to_string());
    }
    
    match mobile_sync::MobileSyncManager::new(config).await {
        Ok(mut sync_manager) => {
            match sync_manager.periodic_sync().await {
                Ok(_) => Ok("Mobile sync completed successfully".to_string()),
                Err(e) => Err(format!("Mobile sync failed: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to create mobile sync manager: {}", e)),
    }
}

#[tauri::command]
#[specta::specta] 
async fn debug_sync_status(_db: tauri::State<'_, Database>, _app: tauri::AppHandle) -> Result<String, String> {
    println!("🔍 DEBUG: debug_sync_status called");
    
    // Mobile app is now standalone - no shared config
    let mut status = String::new();
    status.push_str("Mobile sync debugging (standalone mode):\n");
    status.push_str("- Shared config: DISABLED\n");
    status.push_str("- Mobile config: TODO\n");
    Ok(status)
}

#[tauri::command]
#[specta::specta]
async fn test_sync_connection() -> Result<String, String> {
    println!("🔍 DEBUG: test_sync_connection called");
    
    // Get server URL from saved sync config
    let sync_config = auth::get_sync_config().map_err(|e| e.to_string())?;
    let server_url = sync_config.0; // server_url is the first element of the tuple
    
    println!("🔍 DEBUG: Server URL from config: '{}'", server_url);
    
    if server_url.is_empty() {
        return Err("Server URL not configured".to_string());
    }
    
    // Get current config and check sync status
    let config = auth::get_current_mobile_config();
    println!("🔍 DEBUG: Full sync config status:");
    println!("  - JWT valid: {}", config.is_jwt_valid());
    
    // Test basic HTTP connectivity
    let base_url = server_url
        .replace("ws://", "http://")
        .replace("wss://", "https://")
        .replace("/api/sync", "");
    
    println!("🔍 DEBUG: Testing connection to: {}/api/health", base_url);
    
    let client = reqwest::Client::new();
    match client.get(&format!("{}/api/health", base_url)).send().await {
        Ok(response) => {
            println!("🔍 DEBUG: Server response status: {}", response.status());
            if response.status().is_success() {
                Ok("Server connection successful!".to_string())
            } else {
                Err(format!("Server returned status: {}", response.status()))
            }
        }
        Err(e) => {
            println!("🔍 DEBUG: Connection error: {}", e);
            Err(format!("Failed to connect to server: {}", e))
        }
    }
}



// Minimal version for crash debugging - commented out to avoid duplicate symbol
// #[allow(dead_code)]
// pub fn run_minimal() {
//     println!("🚀 Starting minimal lst-mobile...");
//     
//     tauri::Builder::default()
//         .setup(|_app| {
//             println!("✅ Tauri setup completed successfully");
//             Ok(())
//         })
//         .invoke_handler(tauri::generate_handler![])
//         .run(tauri::generate_context!())
//         .expect("error while running tauri application");
// }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _builder = Builder::<tauri::Wry>::new()
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
            trigger_sync,
            test_sync_connection,
            debug_sync_status,
            theme::get_current_theme,
            theme::apply_theme,
            theme::list_themes
        ]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    {
        use specta_typescript::Typescript;
        _builder
            .export(Typescript::default(), "../src/bindings.ts")
            .expect("Failed to export typescript bindings");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            
            // Initialize database with error handling
            let db = match Database::new(&handle) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to initialize database: {}", e);
                    return Err(e.into());
                }
            };
            // Initialize sync config from database
            if let Err(e) = auth::initialize_sync_config_from_db(&db) {
                eprintln!("Failed to initialize sync config from database: {}", e);
                // Non-fatal error, continue with empty config
            } else {
                println!("🔍 DEBUG: Sync config initialized from database");
            }
            
            // Load mobile configuration from database
            if let Err(e) = mobile_config::load_config_from_db(&db) {
                eprintln!("Failed to load mobile config from database: {}", e);
                // Non-fatal error, continue with default config
            } else {
                println!("🔍 DEBUG: Mobile config loaded from database");
                let config = auth::get_current_mobile_config();
                println!("🔍 DEBUG: Mobile configuration status:");
                println!("  - Has syncd: {}", config.has_syncd());
                println!("  - JWT valid: {}", config.is_jwt_valid());
            }
            
            app.manage(db);
            
            // Initialize sync bridge
            let sync_bridge: Arc<Mutex<Option<SyncBridge>>> = Arc::new(Mutex::new(None));
            app.manage(sync_bridge);
            
            // Get window with fallback for mobile
            let _window = app.get_webview_window("main");

            // Start command server and sync on desktop platforms
            #[cfg(not(mobile))]
            {
                command_server::start_command_server(app.handle().clone());
                theme::broadcast_theme(&app.handle()).ok();
            }

            // Start sync service on all platforms (including mobile)
            tauri::async_runtime::spawn(async move {
                println!("🔄 Starting mobile sync service loop...");
                let mut sync_manager: Option<mobile_sync::MobileSyncManager> = None;
                
                loop {
                    println!("🔄 Mobile sync loop iteration starting...");
                    
                    // Get the current config (which may have been updated with JWT)
                    let config = auth::get_current_mobile_config();
                    
                    // Only try to sync if we have sync configuration  
                    if config.has_syncd() && config.is_jwt_valid() {
                        // Create sync manager if we don't have one, or if config changed
                        if sync_manager.is_none() {
                            println!("🔄 Mobile config valid, creating new mobile sync manager...");
                            match mobile_sync::MobileSyncManager::new(config).await {
                                Ok(mgr) => {
                                    println!("🔄 ✅ Mobile sync manager created successfully");
                                    sync_manager = Some(mgr);
                                }
                                Err(e) => {
                                    eprintln!("🔄 ❌ Failed to initialize mobile sync manager: {}", e);
                                    sync_status::mark_sync_disconnected(format!("Init failed: {}", e)).ok();
                                    sync_manager = None;
                                }
                            }
                        }
                        
                        // Run periodic sync if we have a manager
                        if let Some(ref mut mgr) = sync_manager {
                            println!("🔄 Running periodic sync with mobile sync manager...");
                            if let Err(e) = mgr.periodic_sync().await {
                                eprintln!("🔄 ❌ Mobile sync error: {e}");
                                sync_status::mark_sync_disconnected(e.to_string()).ok();
                                // Reset sync manager on error so it gets recreated
                                sync_manager = None;
                            } else {
                                println!("🔄 ✅ Mobile periodic sync completed successfully");
                                sync_status::mark_sync_connected().ok();
                            }
                        }
                    } else {
                        println!("🔄 ⚠️ Mobile sync not configured or JWT expired");
                        sync_status::mark_sync_disconnected("Mobile sync not configured or JWT expired".to_string()).ok();
                        sync_manager = None; // Reset manager if config is invalid
                    }
                    
                    println!("🔄 Mobile sync sleeping for 30 seconds...");
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });

            // Apply vibrancy effects only on desktop platforms
            #[cfg(all(target_os = "macos", not(mobile)))]
            if let Some(window) = _window {
                window_vibrancy::apply_vibrancy(
                    &window,
                    window_vibrancy::NSVisualEffectMaterial::HudWindow,
                    None,
                    Some(5.0),
                ).ok(); // Don't panic on mobile
            }

            #[cfg(all(target_os = "windows", not(mobile)))]
            if let Some(window) = _window {
                window_vibrancy::apply_blur(&window, Some((18, 18, 18, 125))).ok();
            }

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
            trigger_sync,
            test_sync_connection,
            debug_sync_status,
            theme::get_current_theme,
            theme::apply_theme,
            theme::list_themes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
