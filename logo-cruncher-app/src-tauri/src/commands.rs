use logoLoader::{test, Config, Jobs};

#[tauri::command]
pub fn process_json(json: &str) -> Jobs {
    let config = Config::get();
    let logos = Jobs::load_json_job(json, config.job(), &config.temp_job_file(), false)
        .expect("Failed to load JSON job");
    println!("Распарсили заданий {}", logos.logos.len());
    // println!("Привет от Json из Rust2! {json} {:?}", logos);
    let result = test(json);
    println!("Привет от Json из Rust2 process_json! {result} dd");
    logos
}
