// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[tauri::command]
fn greet(name: &str) -> String {
    println!("Привет из раста!");

    format!("Hello, {}!Привет из раста!", name)
}

#[tauri::command]
fn logo_list(logos_dir: String) -> String {
    println!("Хочу получить список файлов тут: {logos_dir}");
    format!("Хочу получить список файлов тут: {logos_dir}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, logo_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
