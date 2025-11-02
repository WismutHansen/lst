use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ThemeData {
    pub css_variables: String,
    pub scheme: String,
    pub name: Option<String>,
    pub variant: Option<String>,
}

/// Get the current theme and generate CSS variables
#[tauri::command]
#[specta::specta]
pub fn get_current_theme() -> Result<ThemeData, String> {
    // Reload config from disk to get latest theme changes from CLI
    let config = lst_cli::config::Config::load().map_err(|e| e.to_string())?;
    let theme = config.get_theme().map_err(|e| e.to_string())?;

    Ok(ThemeData {
        css_variables: theme.generate_css_variables(),
        scheme: theme.scheme.clone(),
        name: theme.name.clone(),
        variant: theme.variant.as_ref().map(|v| format!("{:?}", v)),
    })
}

/// Apply a theme by name
#[tauri::command]
#[specta::specta]
pub fn apply_theme(theme_name: String) -> Result<ThemeData, String> {
    let mut config = lst_cli::config::Config::load().map_err(|e| e.to_string())?;
    let theme = config
        .load_theme_by_name(&theme_name)
        .map_err(|e| e.to_string())?;

    config.set_theme(theme.clone());
    config.save().map_err(|e| e.to_string())?;

    Ok(ThemeData {
        css_variables: theme.generate_css_variables(),
        scheme: theme.scheme.clone(),
        name: theme.name.clone(),
        variant: theme.variant.as_ref().map(|v| format!("{:?}", v)),
    })
}

/// List all available themes
#[tauri::command]
#[specta::specta]
pub fn list_themes() -> Result<Vec<String>, String> {
    // Reload config to get latest themes directory configuration
    let config = lst_cli::config::Config::load().map_err(|e| e.to_string())?;
    let loader = config.get_theme_loader();
    Ok(loader.list_themes())
}

/// Broadcast theme update to frontend
pub fn broadcast_theme(app: &AppHandle) -> tauri::Result<()> {
    match get_current_theme() {
        Ok(theme_data) => {
            app.emit("theme-update", theme_data)?;
        }
        Err(e) => {
            eprintln!("Failed to get current theme: {}", e);
        }
    }
    Ok(())
}
