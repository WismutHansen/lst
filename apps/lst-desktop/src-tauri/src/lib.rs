use anyhow::Result;
use lst_cli::config::{get_config, UiConfig};
use lst_cli::models::{fuzzy_find, is_valid_anchor, ItemStatus, List};
use lst_cli::storage::{
    list_lists, list_notes,
    markdown::{self, load_list},
};
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

#[tauri::command]
#[specta::specta]
fn create_list(title: String) -> Result<List, String> {
    Ok(List::new(title))
}

#[tauri::command]
#[specta::specta]
fn add_item(list: String, text: String) -> Result<List, String> {
    // create list if missing
    if load_list(&list).is_err() {
        markdown::create_list(&list).map_err(|e| e.to_string())?;
    }
    for item in text.split(',').map(|s| s.trim()) {
        if !item.is_empty() {
            markdown::add_item(&list, item).map_err(|e| e.to_string())?;
        }
    }
    load_list(&list).map_err(|e| e.to_string())
}

fn find_item_index(list: &List, target: &str) -> Option<usize> {
    if is_valid_anchor(target) {
        if let Some(idx) = list.find_by_anchor(target) {
            return Some(idx);
        }
    }
    if let Some(idx) = list.find_by_text(target) {
        return Some(idx);
    }
    if let Some(number_str) = target.strip_prefix('#') {
        if let Ok(num) = number_str.parse::<usize>() {
            if let Some(item) = list.get_by_index(num - 1) {
                if let Some(idx) = list.find_by_anchor(&item.anchor) {
                    return Some(idx);
                }
            }
        }
    }
    let matches = fuzzy_find(&list.items, target, 0.75);
    match matches.len() {
        1 => Some(matches[0]),
        _ => None,
    }
}

#[tauri::command]
#[specta::specta]
fn toggle_item(list: String, target: String) -> Result<List, String> {
    let current = load_list(&list).map_err(|e| e.to_string())?;
    if let Some(idx) = find_item_index(&current, &target) {
        let status = current.items[idx].status.clone();
        drop(current);
        match status {
            ItemStatus::Todo => {
                markdown::mark_done(&list, &target).map_err(|e| e.to_string())?;
            }
            ItemStatus::Done => {
                markdown::mark_undone(&list, &target).map_err(|e| e.to_string())?;
            }
        }
        load_list(&list).map_err(|e| e.to_string())
    } else {
        Err(format!("No item matching '{}'", target))
    }
}

#[tauri::command]
#[specta::specta]
fn remove_item(list: String, target: String) -> Result<List, String> {
    let current = load_list(&list).map_err(|e| e.to_string())?;
    if let Some(_idx) = find_item_index(&current, &target) {
        drop(current);
        markdown::delete_item(&list, &target).map_err(|e| e.to_string())?;
        load_list(&list).map_err(|e| e.to_string())
    } else {
        Err(format!("No item matching '{}'", target))
    }
}

#[tauri::command]
#[specta::specta]
fn get_ui_config() -> Result<UiConfig, String> {
    Ok(get_config().ui.clone())
}

#[tauri::command]
#[specta::specta]
fn edit_item(list: String, target: String, text: String) -> Result<List, String> {
    markdown::edit_item_text(&list, &target, &text).map_err(|e| e.to_string())?;
    load_list(&list).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn reorder_item(list: String, target: String, new_index: u32) -> Result<List, String> {
    markdown::reorder_item(&list, &target, new_index as usize).map_err(|e| e.to_string())?;
    load_list(&list).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn save_list(list: List) -> Result<(), String> {
    markdown::save_list(&list).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        // Then register them (separated by a comma)
        .commands(collect_commands![
            get_lists,
            get_notes,
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
            let _tray = TrayIconBuilder::new().build(app)?;
            let _window = app.get_webview_window("main").unwrap();

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
