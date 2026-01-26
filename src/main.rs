use config::{CROP_FOLDER, Config, DOWNLOAD_FOLDER, LOG_FILE, RESULT_FOLDER, UPSCALE_FOLDER};
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
    let out_dir_path = Path::new(&config.out_dir);

    // Инициализация лога
    println!("Инициализация лога");
    setup_logger(&Path::new(&config.out_dir).join(LOG_FILE));

    // Скачка задания
    println!("Скачка задания {}", job_path.to_str().unwrap());
    // let logos = Jobs::generate_job_from_dir_images(DOWNLOAD_FOLDER);
    // let logos = loaders::simple_load_job(JSON_FILE_PATH)?;
    let logos = Jobs::load_json_job("jobs_from_advisa.json").await;

    // Создаем папки если нет
    for folder in &[DOWNLOAD_FOLDER, UPSCALE_FOLDER, RESULT_FOLDER, CROP_FOLDER] {
        create_dir(*folder)?;
    }

    // Скачка файлов  картинок задания
    if config.download {
        // download_images(logos.clone()).await;
    }

    // Обрезка краев в один пиксель
    // remove_border_parallel(logos.clone()).await?;

    // Увеличение разрешения файлов
    if config.upscale {
        // image_worker::upscale_images().await?;
    }
    // Обработка файлов
    // images_works_parallel(logos.clone()).await?;

    Ok(())
}

// Создать директорию если ее не существует
pub fn create_dir(dir: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !Path::new(dir).exists() {
        fs::create_dir_all(dir)?;
        info!("Создана директория: {}", dir);
    }
    Ok(())
}
