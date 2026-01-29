use logoLoader::{
    create_dir, download_images, images_works_parallel, remove_border_parallel, setup_logger,
    upscale_images, Config, Jobs,
};
use std::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = Config::get();
    let job_path = Path::new(config.job());

    println!("Инициализация лога");
    setup_logger(&config.log_file());

    println!("Скачка задания {}", job_path.to_str().unwrap());
    // let logos = Jobs::generate_job_from_dir_images(config.download_folder().to_str().unwrap());
    // let logos = loaders::simple_load_job(JSON_FILE_PATH)?;
    let logos = Jobs::load_json_job(config.job(), Some(&config.temp_job_file())).await;

    for folder in config.all_folders() {
        create_dir(&folder)?;
    }

    if config.download() {
        download_images(logos.clone(), &config).await;
    }

    remove_border_parallel(logos.clone(), &config).await?;

    if config.upscale() {
        upscale_images(&config).await?;
    }
    images_works_parallel(logos.clone(), &config).await?;

    Ok(())
}
