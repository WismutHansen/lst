use lst_cli::config::get_config;
use tauri::{AppHandle, Emitter};

pub fn broadcast_theme(app: &AppHandle) -> tauri::Result<()> {
    let theme = get_config().ui.theme.clone();
    // Only emit if there are any variables to set
    if !theme.vars.is_empty() {
        app.emit("theme-update", theme)?;
    }
    Ok(())
}
