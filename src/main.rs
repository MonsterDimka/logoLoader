use logoLoader::{
    create_dir, delete_dir, download_images, images_works_parallel, remove_border_parallel,
    setup_logger, upscale_images, Config, Jobs,
};
use std::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = Config::get();

    println!("Инициализация лога");
    setup_logger(&config.log_file())?;

    // let logos = loaders::simple_load_job(JSON_FILE_PATH)?;
    for folder in config.all_folders() {
        create_dir(&folder)?;
    }

    println!("Скачка задания {}", Path::new(config.job()).display());
    let logos = Jobs::load_json_job("", config.job(), &config.temp_job_file(), true)?;

    if config.download() {
        download_images(&logos, &config).await?;
    }

    let logos =
        Jobs::generate_job_from_dir_images(&config.download_folder().display().to_string())?;

    remove_border_parallel(&logos, &config).await?;

    if config.upscale() {
        upscale_images(&config).await?;
    }
    images_works_parallel(&logos, &config).await?;

    for folder in config.clean_folders() {
        delete_dir(&folder)?;
    }

    Ok(())
}
