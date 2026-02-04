mod background_works;
mod config;
mod image_loader;
mod image_worker;
mod job_loaders;
mod logger;
mod parsers;
mod svg_saver;
mod vectorize;

pub use config::Config;
pub use image_loader::download_images;
pub use image_worker::{images_works_parallel, remove_border_parallel, upscale_images};
pub use job_loaders::{Jobs, LogoJob};
pub use logger::setup_logger;

pub fn create_dir(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
        log::info!("Создана директория: {}", dir.display());
    }
    Ok(())
}

pub fn test(str: &str) -> String {
    let mut res = format!("Тестовый вызов библиотеки {str}");
    res.truncate(100);
    res
}
