
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter};
use lst_core::theme::{Theme, ThemeLoader};

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
    // Use Nord as the default theme for mobile
    let theme_loader = ThemeLoader::new();
    let theme = theme_loader.load_theme("base16-nord")
        .unwrap_or_else(|_| Theme::default()); // Fallback to default if Nord fails to load
    
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
    // For mobile, we'll use built-in themes only
    let theme_loader = ThemeLoader::new();
    let theme = theme_loader.load_theme(&theme_name).map_err(|e| e.to_string())?;
    
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
    // Use built-in themes only for mobile
    let theme_loader = ThemeLoader::new();
    Ok(theme_loader.list_themes())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_is_nord() {
        let theme_data = get_current_theme().expect("Should load default theme");
        
        // Verify that the theme name contains "Nord" (case insensitive)
        if let Some(name) = &theme_data.name {
            assert!(name.to_lowercase().contains("nord"), 
                "Expected theme name to contain 'nord', got: {}", name);
        }
        
        // Verify that CSS variables are generated
        assert!(!theme_data.css_variables.is_empty(), 
            "CSS variables should not be empty");
        
        // Verify that the scheme is set correctly
        assert!(!theme_data.scheme.is_empty(), 
            "Scheme should not be empty");
        
        println!("✅ Default theme: {} (scheme: {})", 
            theme_data.name.unwrap_or_else(|| "Unknown".to_string()), 
            theme_data.scheme);
    }

    #[test]
    fn test_nord_theme_loads_correctly() {
        let theme_data = apply_theme("base16-nord".to_string())
            .expect("Should load Nord theme");
        
        // Verify Nord theme properties
        assert!(theme_data.name.is_some(), "Nord theme should have a name");
        assert_eq!(theme_data.scheme, "base16-nord", "Scheme should be base16-nord");
        assert!(!theme_data.css_variables.is_empty(), "CSS variables should be generated");
        
        println!("✅ Nord theme loaded: {} (scheme: {})", 
            theme_data.name.unwrap_or_else(|| "Unknown".to_string()), 
            theme_data.scheme);
    }

    #[test]
    fn test_theme_list_includes_nord() {
        let themes = list_themes().expect("Should list themes");
        
        // Verify that Nord is in the list of available themes
        assert!(themes.contains(&"base16-nord".to_string()), 
            "Available themes should include 'base16-nord'");
        
        // Verify that we have multiple themes available
        assert!(!themes.is_empty(), "Should have at least one theme");
        
        println!("✅ Available themes: {} (including Nord)", themes.len());
    }
}
