// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn greet(name: &str) -> String {
    println!("Привет из раста!");

    format!("Hello, {}!Привет из раста!", name)
}

#[tauri::command]
async fn get_file_list(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let folder_path = app
        .dialog()
        .file()
        .blocking_pick_folder()
        .ok_or("Директория не выбрана")?;

    let path_buf = folder_path
        .into_path()
        .map_err(|e| format!("Ошибка пути: {}", e))?;

    let entries =
        fs::read_dir(&path_buf).map_err(|e| format!("Ошибка чтения директории: {}", e))?;

    let file_list: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                path.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    Ok(file_list)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_file_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
