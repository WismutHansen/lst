// Minimal version for crash debugging
use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TestResponse {
    pub message: String,
    pub success: bool,
}

#[tauri::command]
#[specta::specta]
fn test_command() -> Result<TestResponse, String> {
    Ok(TestResponse {
        message: "Test command working".to_string(),
        success: true,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("🚀 Starting minimal lst-mobile...");
    
    tauri::Builder::default()
        .setup(|app| {
            println!("✅ Tauri setup completed successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![test_command])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}