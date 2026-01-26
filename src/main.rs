use config::Config;
use image_loader::download_images;
use image_worker::{images_works_parallel, remove_border_parallel};
use job_loaders::Jobs;
use log::info;
use logger::setup_logger;
use std::error::Error;
use std::fs;
use std::path::Path;
mod background_works;
mod config;
mod image_loader;
mod image_worker;
mod job_loaders;
mod logger;
mod parsers;
mod save;
mod vectorize;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = Config::get();
    let job_path = Path::new(&config.job);

    // Инициализация лога
    println!("Инициализация лога");
    setup_logger(&config.log_file());

    // Скачка задания
    println!("Скачка задания {}", job_path.to_str().unwrap());
    // let logos = Jobs::generate_job_from_dir_images(config.download_folder().to_str().unwrap());
    // let logos = loaders::simple_load_job(JSON_FILE_PATH)?;
    let logos = Jobs::load_json_job("jobs_from_advisa.json", Some(&config.temp_job_file())).await;

    // Создаем папки если нет
    for folder in config.all_folders() {
        create_dir(&folder)?;
    }

    // Скачка файлов  картинок задания
    if config.download {
        // download_images(logos.clone(), &config).await;
    }

    // Обрезка краев в один пиксель
    // remove_border_parallel(logos.clone(), &config).await?;

    // Увеличение разрешения файлов
    if config.upscale {
        // image_worker::upscale_images(&config).await?;
    }
    // Обработка файлов
    // images_works_parallel(logos.clone(), &config).await?;

    Ok(())
}

// Создать директорию если ее не существует
pub fn create_dir(dir: &std::path::Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
        info!("Создана директория: {}", dir.display());
    }
    Ok(())
}
