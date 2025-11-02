use anyhow::Result;
use chrono;
use lst_cli::config::{get_config, UiConfig};
use lst_cli::models::{fuzzy_find, is_valid_anchor, ItemStatus, List, ListItem};
use lst_cli::storage::{
    list_lists, list_notes,
    markdown::{self, load_list},
    notes::{create_note, delete_note, load_note},
};
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Typescript;
use std::fs;
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
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
    let list = List::new(title);
    markdown::save_list(&list).map_err(|e| e.to_string())?;
    Ok(list)
}

#[tauri::command]
#[specta::specta]
fn add_item(list: String, text: String, category: Option<String>) -> Result<List, String> {
    // create list if missing
    if load_list(&list).is_err() {
        markdown::create_list(&list).map_err(|e| e.to_string())?;
    }

    for item in text.split(',').map(|s| s.trim()) {
        if !item.is_empty() {
            // Check for ##category inline syntax
            let (parsed_category, parsed_text) = parse_item_input(item);
            let final_category = parsed_category.or(category.as_deref());

            markdown::add_item_to_category(&list, parsed_text, final_category)
                .map_err(|e| e.to_string())?;
        }
    }
    load_list(&list).map_err(|e| e.to_string())
}

fn parse_item_input(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("##") {
        if let Some(space_index) = input.find(' ') {
            if space_index > 2 {
                let category = &input[2..space_index];
                let text = &input[space_index + 1..];
                return (Some(category), text);
            }
        }
    }
    (None, input)
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
    let all_items: Vec<ListItem> = list.all_items().cloned().collect();
    let matches = fuzzy_find(&all_items, target, 75);
    match matches.len() {
        1 => Some(matches[0]),
        _ => None,
    }
}

#[tauri::command]
#[specta::specta]
fn toggle_item(list: String, target: String) -> Result<List, String> {
    let config = get_config();
    let current = load_list(&list).map_err(|e| e.to_string())?;
    if let Some(idx) = find_item_index(&current, &target) {
        let status = current.all_items().nth(idx).unwrap().status.clone();
        drop(current);
        match status {
            ItemStatus::Todo => {
                markdown::mark_done(&list, &target, config.fuzzy.threshold)
                    .map_err(|e| e.to_string())?;
            }
            ItemStatus::Done => {
                markdown::mark_undone(&list, &target, config.fuzzy.threshold)
                    .map_err(|e| e.to_string())?;
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
    let config = get_config();
    let current = load_list(&list).map_err(|e| e.to_string())?;
    if let Some(_idx) = find_item_index(&current, &target) {
        drop(current);
        println!("deleting item {}", target);
        markdown::delete_item(&list, &target, config.fuzzy.threshold).map_err(|e| e.to_string())?;
        load_list(&list).map_err(|e| e.to_string())
    } else {
        Err(format!("No item matching '{}'", target))
    }
}

#[tauri::command]
#[specta::specta]
fn get_note(name: String) -> Result<Note, String> {
    let path = load_note(&name).map_err(|e| e.to_string())?;
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    // Parse frontmatter to extract title and created date
    let (title, created, content_without_frontmatter) = parse_note_frontmatter(&content, &name);

    Ok(Note {
        title,
        content: content_without_frontmatter,
        created,
        file_path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
#[specta::specta]
fn create_note_cmd(title: String) -> Result<Note, String> {
    let path = create_note(&title).map_err(|e| e.to_string())?;
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let (parsed_title, created, content_without_frontmatter) =
        parse_note_frontmatter(&content, &title);

    Ok(Note {
        title: parsed_title,
        content: content_without_frontmatter,
        created,
        file_path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
#[specta::specta]
fn save_note(note: Note) -> Result<(), String> {
    let path = std::path::Path::new(&note.file_path);

    // Build content with frontmatter
    let frontmatter = if let Some(created) = &note.created {
        format!(
            "---\ntitle: \"{}\"\ncreated: {}\n---\n\n",
            note.title, created
        )
    } else {
        format!("---\ntitle: \"{}\"\n---\n\n", note.title)
    };

    let full_content = format!("{}{}", frontmatter, note.content);
    fs::write(path, full_content).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
fn delete_note_cmd(name: String) -> Result<(), String> {
    delete_note(&name).map_err(|e| e.to_string())
}

fn parse_note_frontmatter(content: &str, fallback_title: &str) -> (String, Option<String>, String) {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0] != "---" {
        return (fallback_title.to_string(), None, content.to_string());
    }

    let mut frontmatter_end = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            frontmatter_end = Some(i);
            break;
        }
    }

    let Some(end_idx) = frontmatter_end else {
        return (fallback_title.to_string(), None, content.to_string());
    };

    let mut title = fallback_title.to_string();
    let mut created = None;

    // Parse frontmatter
    for line in &lines[1..end_idx] {
        if let Some(title_value) = line.strip_prefix("title: ") {
            title = title_value.trim_matches('"').to_string();
        } else if let Some(created_value) = line.strip_prefix("created: ") {
            created = Some(created_value.to_string());
        }
    }

    // Get content after frontmatter
    let content_lines = if end_idx + 1 < lines.len() {
        &lines[end_idx + 1..]
    } else {
        &[]
    };

    let content_without_frontmatter = content_lines.join("\n").trim_start().to_string();

    (title, created, content_without_frontmatter)
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
    let config = get_config();
    markdown::reorder_item(&list, &target, new_index as usize, config.fuzzy.threshold)
        .map_err(|e| e.to_string())?;
    load_list(&list).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn save_list(list: List) -> Result<(), String> {
    markdown::save_list(&list).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
fn create_category(list_name: String, category_name: String) -> Result<List, String> {
    let mut list = load_list(&list_name).map_err(|e| e.to_string())?;

    // Check if category already exists
    if list.categories.iter().any(|c| c.name == category_name) {
        return Err(format!("Category '{}' already exists", category_name));
    }

    // Add new empty category
    list.categories.push(lst_cli::models::Category {
        name: category_name,
        items: Vec::new(),
    });

    list.metadata.updated = chrono::Utc::now();
    markdown::save_list_with_path(&list, &list_name).map_err(|e| e.to_string())?;
    Ok(list)
}

#[tauri::command]
#[specta::specta]
fn move_item_to_category(
    list_name: String,
    item_anchor: String,
    category_name: Option<String>,
) -> Result<List, String> {
    let config = get_config();
    let mut list = load_list(&list_name).map_err(|e| e.to_string())?;

    // Find and remove the item from its current location
    let item_location =
        markdown::find_item_for_removal(&list, &item_anchor, config.fuzzy.threshold)
            .map_err(|e| e.to_string())?;
    let item = markdown::remove_item_at_location(&mut list, item_location);

    // Add item to new location
    match category_name {
        Some(cat_name) => {
            // Find or create category
            if let Some(category) = list.categories.iter_mut().find(|c| c.name == cat_name) {
                category.items.push(item);
            } else {
                // Create new category
                list.categories.push(lst_cli::models::Category {
                    name: cat_name,
                    items: vec![item],
                });
            }
        }
        None => {
            // Move to uncategorized
            list.uncategorized_items.push(item);
        }
    }

    list.metadata.updated = chrono::Utc::now();
    markdown::save_list_with_path(&list, &list_name).map_err(|e| e.to_string())?;
    Ok(list)
}

#[tauri::command]
#[specta::specta]
fn delete_category(list_name: String, category_name: String) -> Result<List, String> {
    let mut list = load_list(&list_name).map_err(|e| e.to_string())?;

    // Find category and move its items to uncategorized
    if let Some(pos) = list.categories.iter().position(|c| c.name == category_name) {
        let category = list.categories.remove(pos);
        list.uncategorized_items.extend(category.items);

        list.metadata.updated = chrono::Utc::now();
        markdown::save_list_with_path(&list, &list_name).map_err(|e| e.to_string())?;
        Ok(list)
    } else {
        Err(format!("Category '{}' not found", category_name))
    }
}

#[tauri::command]
#[specta::specta]
fn get_categories(list_name: String) -> Result<Vec<String>, String> {
    let list = load_list(&list_name).map_err(|e| e.to_string())?;
    Ok(list.categories.iter().map(|c| c.name.clone()).collect())
}

#[tauri::command]
#[specta::specta]
fn rename_category(list_name: String, old_name: String, new_name: String) -> Result<List, String> {
    let mut list = load_list(&list_name).map_err(|e| e.to_string())?;

    // Check if new name already exists
    if list.categories.iter().any(|c| c.name == new_name) {
        return Err(format!("Category '{}' already exists", new_name));
    }

    // Find and rename category
    if let Some(category) = list.categories.iter_mut().find(|c| c.name == old_name) {
        category.name = new_name;
        list.metadata.updated = chrono::Utc::now();
        markdown::save_list_with_path(&list, &list_name).map_err(|e| e.to_string())?;
        Ok(list)
    } else {
        Err(format!("Category '{}' not found", old_name))
    }
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
            get_note,
            create_note_cmd,
            save_note,
            delete_note_cmd,
            get_ui_config,
            create_category,
            move_item_to_category,
            delete_category,
            get_categories,
            rename_category,
            theme::get_current_theme,
            theme::apply_theme,
            theme::list_themes
        ]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(not(target_os = "ios"))]
            let _tray = TrayIconBuilder::new().build(app)?;
            let _window = app.get_webview_window("main").unwrap();

            command_server::start_command_server(app.handle().clone());
            theme::broadcast_theme(&app.handle()).ok();

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
            get_note,
            create_note_cmd,
            save_note,
            delete_note_cmd,
            get_ui_config,
            create_category,
            move_item_to_category,
            delete_category,
            get_categories,
            rename_category,
            theme::get_current_theme,
            theme::apply_theme,
            theme::list_themes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
