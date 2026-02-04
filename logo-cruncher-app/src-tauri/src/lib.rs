use logoLoader::{test, Config, Jobs, LogoJob};
use std::fs;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn greet(app: AppHandle, name: &str) -> String {
    println!("Привет из раста! {}", name);
    let res = format!("Hello, {}! Привет из раста!", name);
    app.emit("event-greet-finished", &res).unwrap();
    res
}
#[tauri::command]
// fn process_json(json: &str) -> Result<Jobs, String> {
// fn process_json(json: &str) -> Jobs {
fn process_json(json: &str) -> Jobs {
    let config = Config::get();
    let logos = Jobs::load_json_job(json, config.job(), &config.temp_job_file(), false);
    println!("Распарсили заданий {}", logos.logos.len());
    // println!("Привет от Json из Rust2! {json} {:?}", logos);
    let result = test(json);
    println!("Привет от Json из Rust2 process_json! {result} dd");
    logos
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
        .invoke_handler(tauri::generate_handler![greet, get_file_list, process_json])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
